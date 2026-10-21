#[test]
fn the_embedded_mcmurry_logo_decodes_with_the_installed_loaders() {
    let context = egui::Context::default();
    egui_extras::install_image_loaders(&context);
    let uri = "bytes://mcmurrypdf.png";
    context.include_bytes(uri, include_bytes!("../../../assets/mcmurry/mcmurrypdf.png").as_slice());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match context.try_load_image(uri, egui::load::SizeHint::default()) {
            Ok(egui::load::ImagePoll::Ready { image }) => {
                assert_eq!(image.size, [1254, 1254]);
                break;
            }
            Err(error) => panic!("Embedded McMurryPDF logo did not decode: {error}"),
            Ok(_) => {
                assert!(std::time::Instant::now() < deadline, "Embedded McMurryPDF logo remained pending");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
