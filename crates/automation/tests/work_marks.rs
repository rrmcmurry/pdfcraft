use pdfcraft_automation::{Automation, Content};
use serde_json::{Value, json};

fn call(a: &mut Automation, name: &str, args: Value) -> Value {
    let response = a.call(name, &args).unwrap();
    match &response[0] {
        Content::Json(v) => v.clone(),
        other => panic!("Expected JSON, got {other:?}"),
    }
}

#[test]
fn mark_tool_round_trips_comments_without_becoming_fill_and_sign() {
    let dir = std::env::temp_dir().join(format!("mcmurry-work-marks-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut a = Automation::new().with_root(&dir).unwrap().with_clock(|| 1_700_000_000);
    let doc = call(&mut a, "doc_create", json!({"from":"blank"}))["doc"].as_u64().unwrap();
    for (n, text) in [".", "/", "B", "4797", "D", "WTB", "A/B"].iter().enumerate() {
        call(&mut a, "mark_add", json!({"doc":doc,"page":1,"at":[100,100+n*30],"text":text,"author":"Kelly","color":"blue","font":"Courier-Bold"}));
    }
    let comments = call(&mut a, "comment_list", json!({"doc":doc}));
    let serialized = comments.to_string();
    for label in ["•", "✓", "B", "4797", "WTB", "A/B", "Kelly"] {
        assert!(serialized.contains(label), "missing {label}: {serialized}");
    }
    for bad in ["", "\nB", "12345678901234567890123456789012345", "字"] {
        // A newline inside the mark must fail; leading/trailing whitespace is trimmed.
        let bad = if bad == "\nB" { "B\nD" } else { bad };
        assert!(a.call("mark_add", &json!({"doc":doc,"page":1,"at":[100,100],"text":bad})).is_err());
    }
    call(&mut a, "doc_save", json!({"doc":doc,"path":"marked.pdf","flatten_fill_sign":true}));
    assert!(a.call("mark_add", &json!({"doc":doc,"page":1,"at":[100,100],"text":"B","font":"unknown-font"})).is_err());
    let saved = pdfcraft_cos::Document::open(std::sync::Arc::new(std::fs::read(dir.join("marked.pdf")).unwrap())).unwrap();
    assert!(saved.scan_objects().any(|(_, object)| object.as_dict().is_some_and(|d| d.name(b"BaseFont") == Some(b"Courier-Bold"))));
    let reopened = call(&mut a, "doc_open", json!({"path":"marked.pdf"}))["doc"].as_u64().unwrap();
    let again = call(&mut a, "comment_list", json!({"doc":reopened}));
    assert_eq!(comments["comments"].as_array().unwrap().len(), 7);
    assert_eq!(again["comments"].as_array().unwrap().len(), 7);
    for label in ["•", "✓", "4797", "Kelly"] {
        assert!(again.to_string().contains(label));
    }
    // Root confinement still applies to the document save path.
    assert!(a.call("doc_save", &json!({"doc":doc,"path":"../outside-marks.pdf"})).is_err());
    std::fs::remove_file(dir.join("marked.pdf")).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}
