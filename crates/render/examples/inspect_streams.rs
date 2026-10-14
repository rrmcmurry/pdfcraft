//! Local read-only parser diagnosis; prints structure, never document text.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    const LIMIT: u64 = 512 * 1024 * 1024;
    let path = std::env::args().nth(1).ok_or("expected PDF path")?;
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > LIMIT {
        return Err("PDF exceeds the 512 MiB diagnostic limit".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err("PDF exceeds the 512 MiB diagnostic limit".into());
    }
    let pdf = hayro::hayro_syntax::Pdf::new(bytes).map_err(|error| format!("{error:?}"))?;
    for (index, page) in pdf.pages().iter().enumerate() {
        match page.page_stream() {
            Some(stream) => println!("page {} decoded bytes: {}", index + 1, stream.len()),
            None => println!("page {} has no decodable content stream", index + 1),
        }
        let mut reader = page.content_reader();
        let mut decoded = Vec::new();
        while decoded.len() < 1024 * 1024 && reader.read_into(&mut decoded, 4096) != 0 {}
        println!("page {} incremental decoded bytes: {}", index + 1, decoded.len());
    }
    // Reproduce only the stream-header syntax with original, in-memory content.
    for ending in ["\r\n", " \r\n"] {
        let content = "0 g 0 0 20 20 re f\n";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Contents 4 0 R >>".to_owned(),
            format!("<< /Length {} >> stream{ending}{content}endstream", content.len()),
        ];
        let mut bytes = b"%PDF-1.3\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
        }
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        let fixture = hayro::hayro_syntax::Pdf::new(bytes).map_err(|error| format!("{error:?}"))?;
        for page in fixture.pages().iter() {
            println!("stream ending {ending:?}: decoded content length {:?}", page.page_stream().map(<[u8]>::len));
        }
    }
    Ok(())
}
