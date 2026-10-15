// McMurry Software original Windows integration. MIT OR Apache-2.0.
// PDF parsing stays in the existing Rust renderer, in a bounded child process.
#define NOMINMAX
#include <windows.h>
#include <thumbcache.h>
#include <propsys.h>
#include <shlwapi.h>
#include <shlobj.h>
#include <atomic>
#include <algorithm>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>
#include <new>

// {8BE52C39-5E5E-4D32-A663-D1BEB26F65AD}
const CLSID providerId = {0x8be52c39,0x5e5e,0x4d32,{0xa6,0x63,0xd1,0xbe,0xb2,0x6f,0x65,0xad}};
HMODULE moduleHandle = nullptr;
std::atomic<long> objects{0};
std::atomic_flag rendering = ATOMIC_FLAG_INIT;
struct Handle {
    HANDLE h;
    explicit Handle(HANDLE v=nullptr):h(v){}
    ~Handle(){if(h && h!=INVALID_HANDLE_VALUE)CloseHandle(h);}
    Handle(const Handle&)=delete;
    Handle& operator=(const Handle&)=delete;
};
struct RenderGate { ~RenderGate(){rendering.clear();} };
struct TempFiles {
    std::wstring dir;
    ~TempFiles(){if(!dir.empty()){DeleteFileW((dir+L"\\input.pdf").c_str());DeleteFileW((dir+L"\\page.pam").c_str());RemoveDirectoryW(dir.c_str());}}
};
HRESULT render(IStream* stream, UINT cx, HBITMAP* result) {
    if(rendering.test_and_set())return HRESULT_FROM_WIN32(ERROR_BUSY);
    RenderGate gate;
    if(cx==0 || cx>1024)return E_INVALIDARG;
    STATSTG stat{};
    HRESULT hr=stream->Stat(&stat,STATFLAG_NONAME);
    constexpr ULONGLONG limit=512ull*1024*1024;
    if(FAILED(hr))return hr;
    if(stat.cbSize.QuadPart==0 || stat.cbSize.QuadPart>limit)return HRESULT_FROM_WIN32(ERROR_FILE_TOO_LARGE);
    LARGE_INTEGER zero{};
    hr=stream->Seek(zero,STREAM_SEEK_SET,nullptr);
    if(FAILED(hr))return hr;
    wchar_t path[32768];
    DWORD len=GetModuleFileNameW(moduleHandle,path,32768);
    if(!len || len>=32768)return E_FAIL;
    std::wstring dll(path,len);
    auto slash=dll.find_last_of(L"\\");
    if(slash==std::wstring::npos)return E_FAIL;
    std::wstring cli=dll.substr(0,slash)+L"\\McMurryPDF-cli.exe";
    // A DLL can remain loaded after MSI upgrades its versioned folder.
    DWORD rendererBytes=sizeof(path);
    if(RegGetValueW(HKEY_LOCAL_MACHINE,L"Software\\McMurrySoftware\\McMurryPDF",L"ShellRendererPath",RRF_RT_REG_SZ,nullptr,path,&rendererBytes)==ERROR_SUCCESS) {
        if(rendererBytes<sizeof(wchar_t) || rendererBytes>sizeof(path))return E_FAIL;
        path[32767]=L'\0';
        cli=path;
    }
    DWORD tempLen=GetTempPathW(32768,path);
    if(!tempLen || tempLen>=32768)return E_FAIL;
    std::wstring temp(path,tempLen);
    // Windows' isolated host may run at low integrity; use its Low temp folder.
    HANDLE tokenRaw=nullptr;
    if(OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&tokenRaw)) {
        Handle token(tokenRaw);
        DWORD bytes=0;
        GetTokenInformation(token.h,TokenIntegrityLevel,nullptr,0,&bytes);
        std::vector<BYTE> info(bytes);
        if(bytes && GetTokenInformation(token.h,TokenIntegrityLevel,info.data(),bytes,&bytes)) {
            auto label=reinterpret_cast<TOKEN_MANDATORY_LABEL*>(info.data());
            DWORD count=*GetSidSubAuthorityCount(label->Label.Sid);
            if(count && *GetSidSubAuthority(label->Label.Sid,count-1)<SECURITY_MANDATORY_MEDIUM_RID) {
                PWSTR low=nullptr;
                HRESULT folder=SHGetKnownFolderPath(FOLDERID_LocalAppDataLow,KF_FLAG_CREATE,nullptr,&low);
                if(FAILED(folder) || !low)return E_ACCESSDENIED;
                temp=std::wstring(low)+L"\\";
                CoTaskMemFree(low);
            }
        }
    }
    GUID id{};
    if(FAILED(CoCreateGuid(&id)))return E_FAIL;
    wchar_t unique[40];
    if(!StringFromGUID2(id,unique,40))return E_FAIL;
    TempFiles files;
    files.dir=temp+L"McMurryPDF-"+unique;
    if(!CreateDirectoryW(files.dir.c_str(),nullptr)){files.dir.clear();return HRESULT_FROM_WIN32(GetLastError());}
    std::wstring input=files.dir+L"\\input.pdf", output=files.dir+L"\\page.pam";
    {
        Handle file(CreateFileW(input.c_str(),GENERIC_WRITE,0,nullptr,CREATE_NEW,FILE_ATTRIBUTE_TEMPORARY,nullptr));
        if(file.h==INVALID_HANDLE_VALUE)return HRESULT_FROM_WIN32(GetLastError());
        BYTE buffer[65536];
        ULONGLONG total=0;
        while(true) {
            ULONG read=0;
            hr=stream->Read(buffer,sizeof(buffer),&read);
            if(FAILED(hr))return hr;
            if(!read)break;
            total+=read;
            if(total>limit)return HRESULT_FROM_WIN32(ERROR_FILE_TOO_LARGE);
            DWORD written=0;
            if(!WriteFile(file.h,buffer,read,&written,nullptr) || written!=read)return E_FAIL;
        }
    }
    Handle job(CreateJobObjectW(nullptr,nullptr));
    if(!job.h)return E_FAIL;
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits{};
    limits.BasicLimitInformation.LimitFlags=JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE|JOB_OBJECT_LIMIT_PROCESS_MEMORY|JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
    limits.ProcessMemoryLimit=512ull*1024*1024;
    limits.BasicLimitInformation.ActiveProcessLimit=1;
    if(!SetInformationJobObject(job.h,JobObjectExtendedLimitInformation,&limits,sizeof(limits)))return E_FAIL;
    std::wstring command=L"\""+cli+L"\" render \""+input+L"\" --page 1 --dpi 96 --out \""+output+L"\"";
    STARTUPINFOW startup{};
    startup.cb=sizeof(startup);
    PROCESS_INFORMATION pi{};
    if(!CreateProcessW(cli.c_str(),command.data(),nullptr,nullptr,FALSE,CREATE_NO_WINDOW|CREATE_SUSPENDED|BELOW_NORMAL_PRIORITY_CLASS,nullptr,nullptr,&startup,&pi))return HRESULT_FROM_WIN32(GetLastError());
    Handle process(pi.hProcess),thread(pi.hThread);
    if(!AssignProcessToJobObject(job.h,process.h)){TerminateProcess(process.h,1);WaitForSingleObject(process.h,2000);return E_FAIL;}
    if(ResumeThread(thread.h)==static_cast<DWORD>(-1)){TerminateJobObject(job.h,1);WaitForSingleObject(process.h,2000);return E_FAIL;}
    if(WaitForSingleObject(process.h,12000)!=WAIT_OBJECT_0){TerminateJobObject(job.h,1);WaitForSingleObject(process.h,2000);return HRESULT_FROM_WIN32(ERROR_TIMEOUT);}
    DWORD exitCode=1;
    if(!GetExitCodeProcess(process.h,&exitCode) || exitCode!=0)return E_FAIL;
    std::ifstream image(output,std::ios::binary);
    std::string line;
    if(!std::getline(image,line) || line!="P7")return E_FAIL;
    unsigned width=0,height=0,depth=0,maxval=0;
    bool ended=false,rgba=false;
    for(unsigned n=0;n<16 && std::getline(image,line);++n) {
        if(line.size()>100)return E_FAIL;
        if(line=="ENDHDR"){ended=true;break;}
        std::istringstream field(line);
        std::string name;field>>name;
        if(name=="WIDTH")field>>width;
        else if(name=="HEIGHT")field>>height;
        else if(name=="DEPTH")field>>depth;
        else if(name=="MAXVAL")field>>maxval;
        else if(line=="TUPLTYPE RGB_ALPHA")rgba=true;
    }
    if(!ended || !rgba || depth!=4 || maxval!=255 || !width || !height || width>4096 || height>4096)return E_FAIL;
    size_t size=static_cast<size_t>(width)*height*4;
    std::vector<BYTE> pixels(size);
    if(!image.read(reinterpret_cast<char*>(pixels.data()),static_cast<std::streamsize>(size)))return E_FAIL;
    for(size_t n=0;n<size;n+=4)std::swap(pixels[n],pixels[n+2]);
    UINT longest=std::max(width,height);
    UINT w=std::max(1u,width*cx/longest), h=std::max(1u,height*cx/longest);
    BITMAPINFO source{};
    source.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);
    source.bmiHeader.biWidth=static_cast<LONG>(width);
    source.bmiHeader.biHeight=-static_cast<LONG>(height);
    source.bmiHeader.biPlanes=1;source.bmiHeader.biBitCount=32;source.bmiHeader.biCompression=BI_RGB;
    BITMAPINFO dest=source;dest.bmiHeader.biWidth=static_cast<LONG>(w);dest.bmiHeader.biHeight=-static_cast<LONG>(h);
    void* bits=nullptr;
    HBITMAP bitmap=CreateDIBSection(nullptr,&dest,DIB_RGB_COLORS,&bits,nullptr,0);
    if(!bitmap)return E_OUTOFMEMORY;
    HDC dc=CreateCompatibleDC(nullptr);
    if(!dc){DeleteObject(bitmap);return E_FAIL;}
    HGDIOBJ old=SelectObject(dc,bitmap);
    SetStretchBltMode(dc,HALFTONE);SetBrushOrgEx(dc,0,0,nullptr);
    int copied=StretchDIBits(dc,0,0,static_cast<int>(w),static_cast<int>(h),0,0,static_cast<int>(width),static_cast<int>(height),pixels.data(),&source,DIB_RGB_COLORS,SRCCOPY);
    SelectObject(dc,old);DeleteDC(dc);
    if(!copied || copied==GDI_ERROR){DeleteObject(bitmap);return E_FAIL;}
    *result=bitmap;return S_OK;
}
class Thumbnail final : public IInitializeWithStream, public IThumbnailProvider {
    std::atomic<ULONG> refs{1};IStream* stream=nullptr;
public:
    Thumbnail(){++objects;}
    ~Thumbnail(){if(stream)stream->Release();--objects;}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(iid==IID_IUnknown || iid==__uuidof(IInitializeWithStream))*out=static_cast<IInitializeWithStream*>(this);
        else if(iid==__uuidof(IThumbnailProvider))*out=static_cast<IThumbnailProvider*>(this);
        else return E_NOINTERFACE;AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return ++refs;}
    ULONG STDMETHODCALLTYPE Release() override{ULONG n=--refs;if(!n)delete this;return n;}
    HRESULT STDMETHODCALLTYPE Initialize(IStream* value,DWORD) override {
        if(!value)return E_INVALIDARG;if(stream)return HRESULT_FROM_WIN32(ERROR_ALREADY_INITIALIZED);
        stream=value;stream->AddRef();return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetThumbnail(UINT cx,HBITMAP* bitmap,WTS_ALPHATYPE* alpha) override {
        if(!bitmap || !alpha)return E_POINTER;*bitmap=nullptr;*alpha=WTSAT_UNKNOWN;
        if(!stream)return E_UNEXPECTED;
        try {HRESULT hr=render(stream,cx,bitmap);if(SUCCEEDED(hr))*alpha=WTSAT_RGB;return hr;}
        catch(const std::bad_alloc&){return E_OUTOFMEMORY;}catch(...){return E_FAIL;}
    }
};
class Factory final : public IClassFactory {
    std::atomic<ULONG> refs{1};
public:
    Factory(){++objects;}~Factory(){--objects;}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;
        if(iid!=IID_IUnknown && iid!=IID_IClassFactory)return E_NOINTERFACE;
        *out=static_cast<IClassFactory*>(this);AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override{return ++refs;}
    ULONG STDMETHODCALLTYPE Release() override{ULONG n=--refs;if(!n)delete this;return n;}
    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer,REFIID iid,void** out) override {
        if(!out)return E_POINTER;*out=nullptr;if(outer)return CLASS_E_NOAGGREGATION;
        auto object=new(std::nothrow) Thumbnail;if(!object)return E_OUTOFMEMORY;
        HRESULT hr=object->QueryInterface(iid,out);object->Release();return hr;
    }
    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override{if(lock)++objects;else --objects;return S_OK;}
};
extern "C" HRESULT __stdcall DllGetClassObject(REFCLSID clsid,REFIID iid,void** out) {
    if(!out)return E_POINTER;*out=nullptr;if(clsid!=providerId)return CLASS_E_CLASSNOTAVAILABLE;
    auto factory=new(std::nothrow) Factory;if(!factory)return E_OUTOFMEMORY;
    HRESULT hr=factory->QueryInterface(iid,out);factory->Release();return hr;
}
extern "C" HRESULT __stdcall DllCanUnloadNow(){return objects.load()==0?S_OK:S_FALSE;}
BOOL WINAPI DllMain(HINSTANCE instance,DWORD reason,LPVOID){if(reason==DLL_PROCESS_ATTACH)moduleHandle=instance;return TRUE;}
