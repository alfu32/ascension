//! Embedded symbols used by the form widgets; no runtime asset directory is needed.

use std::borrow::Cow;

pub(super) struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let data: &'static [u8] = match path {
            "icons/check.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="m5 12 4 4L19 6" fill="none" stroke="black" stroke-width="2"/></svg>"#,
            "icons/chevron-down.svg" | "icons/chevrons-up-down.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="m6 9 6 6 6-6" fill="none" stroke="black" stroke-width="2"/></svg>"#,
            "icons/chevron-up.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="m6 15 6-6 6 6" fill="none" stroke="black" stroke-width="2"/></svg>"#,
            "icons/close.svg" | "icons/window-close.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="m6 6 12 12M6 18 18 6" fill="none" stroke="black" stroke-width="2"/></svg>"#,
            "icons/search.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><circle cx="10" cy="10" r="6" fill="none" stroke="black" stroke-width="2"/><path d="m15 15 5 5" stroke="black" stroke-width="2"/></svg>"#,
            "icons/window-minimize.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M5 12h14" stroke="black" stroke-width="2"/></svg>"#,
            "icons/window-maximize.svg" | "icons/window-restore.svg" => br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M5 5h14v14H5z" fill="none" stroke="black" stroke-width="2"/></svg>"#,
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(data)))
    }

    fn list(&self, _: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(Vec::new())
    }
}
