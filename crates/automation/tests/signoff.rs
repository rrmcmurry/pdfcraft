use pdfcraft_automation::{Automation, Content};
use serde_json::{Value, json};
fn call(a: &mut Automation, name: &str, args: Value) -> Value {
    match &a.call(name, &args).unwrap()[0] {
        Content::Json(v) => v.clone(),
        other => panic!("Expected JSON: {other:?}"),
    }
}

#[test]
fn capture_and_signoff_are_portable_dated_and_root_confined() {
    let dir = std::env::temp_dir().join(format!("mcmurry-signoff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut a = Automation::new().with_root(&dir).unwrap().with_clock(|| 1792080000);
    let doc = call(&mut a, "doc_create", json!({"from":"blank"}))["doc"].as_u64().unwrap();
    call(&mut a, "mark_add", json!({"doc":doc,"page":1,"at":[100,90],"text":"RM","font_size":36,"color":"black"}));
    call(&mut a, "signoff_capture", json!({"doc":doc,"page":1,"rect":[40,50,160,130],"path":"initials.png"}));
    let image = pdfcraft_engine::SignatureImage::from_bytes(&std::fs::read(dir.join("initials.png")).unwrap()).unwrap();
    assert!(image.rgba().as_chunks::<4>().0.iter().any(|p| p[3] == 0));
    assert!(image.rgba().as_chunks::<4>().0.iter().any(|p| p[3] > 200));
    call(&mut a, "signoff_add", json!({"doc":doc,"page":1,"at":[200,200],"graphic":{"image":"initials.png"},"author":"Kelly"}));
    a = a.with_clock(|| 1792166400);
    call(&mut a, "signoff_add", json!({"doc":doc,"page":1,"at":[200,260],"graphic":{"text":"New"},"author":"Reviewer"}));
    let before = call(&mut a, "comment_list", json!({"doc":doc}));
    for expected in ["Initials — 10/15/26", "Initials — 10/16/26", "Kelly", "Reviewer"] {
        assert!(before.to_string().contains(expected));
    }
    assert!(a.call("signoff_capture", &json!({"doc":doc,"page":1,"rect":[40,50,160,130],"path":"../outside-initials.png"})).is_err());
    assert!(a.call("signoff_add", &json!({"doc":doc,"page":1,"at":[200,200],"graphic":{"image":"../outside-initials.png"}})).is_err());
    assert!(a.call("signoff_capture", &json!({"doc":doc,"page":1,"rect":[200,500,250,550],"path":"empty.png"})).is_err());
    call(&mut a, "doc_save", json!({"doc":doc,"path":"signed-off.pdf","flatten_fill_sign":true}));
    let reopened = call(&mut a, "doc_open", json!({"path":"signed-off.pdf"}))["doc"].as_u64().unwrap();
    let after = call(&mut a, "comment_list", json!({"doc":reopened}));
    assert_eq!(after["comments"].as_array().unwrap().len(), 3);
    assert!(after.to_string().contains("Initials — 10/15/26") && after.to_string().contains("Initials — 10/16/26"));
    for file in ["initials.png", "signed-off.pdf"] {
        std::fs::remove_file(dir.join(file)).unwrap();
    }
    std::fs::remove_dir(dir).unwrap();
}
