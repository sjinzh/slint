// Copyright © SixtyFPS GmbH <info@slint-ui.com>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-commercial

use core::cell::RefCell;

use alloc::rc::Rc;
use std::collections::HashMap;

use crate::lengths::{LogicalLength, ScaleFactor};
use crate::sharedfontdb;

use super::super::PhysicalLength;
use super::vectorfont::VectorFont;

thread_local! {
    static FALLBACK_FONT_ID: once_cell::unsync::Lazy<fontdb::ID> = once_cell::unsync::Lazy::new(|| {
        crate::sharedfontdb::FONT_DB.with(|db| {
            let mut db = db.borrow_mut();
            std::env::var_os("SLINT_DEFAULT_FONT").and_then(|maybe_font_path| {
                let path = std::path::Path::new(&maybe_font_path);
                if path.extension().is_some() {
                    let face_count = db.len();
                    match db.load_font_file(path) {
                        Ok(()) => {
                            db.faces().nth(face_count).map(|face_info| face_info.id)
                        },
                        Err(err) => {
                            eprintln!(
                                "Could not load the font set via `SLINT_DEFAULT_FONT`: {}: {}", path.display(), err,
                            );
                            None
                        },
                    }
                } else {
                    eprintln!(
                        "The environment variable `SLINT_DEFAULT_FONT` is set, but its value is not referring to a file",
                    );
                    None
                }
            }).unwrap_or_else(|| {
                let query = fontdb::Query { families: &[fontdb::Family::SansSerif], ..Default::default() };

                db.query(&query).expect("fatal: fontdb could not locate a sans-serif font on the system")
            })
        })
    })
}

thread_local! {
    static FONTDUE_FONTS: RefCell<HashMap<fontdb::ID, Rc<fontdue::Font>>> = Default::default();
}

fn get_or_create_fontdue_font(fontdb: &fontdb::Database, id: fontdb::ID) -> Rc<fontdue::Font> {
    FONTDUE_FONTS.with(|font_cache| {
        font_cache
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| {
                fontdb
                    .with_face_data(id, |face_data, font_index| {
                        fontdue::Font::from_bytes(
                            face_data,
                            fontdue::FontSettings { collection_index: font_index, scale: 40. },
                        )
                        .expect("fatal: fontdue is unable to parse truetype font")
                        .into()
                    })
                    .unwrap()
            })
            .clone()
    })
}

pub fn match_font(
    request: &super::FontRequest,
    scale_factor: super::ScaleFactor,
) -> Option<VectorFont> {
    request.family.as_ref().and_then(|family_str| {
        let family = fontdb::Family::Name(family_str);

        let query = fontdb::Query { families: &[family], ..Default::default() };

        let requested_pixel_size: PhysicalLength =
            (request.pixel_size.unwrap_or(super::DEFAULT_FONT_SIZE).cast() * scale_factor).cast();

        sharedfontdb::FONT_DB.with(|fonts| {
            let borrowed_fontdb = fonts.borrow();
            borrowed_fontdb.query(&query).map(|font_id| {
                let fontdue_font = get_or_create_fontdue_font(&*borrowed_fontdb, font_id);
                VectorFont::new(font_id, fontdue_font.clone(), requested_pixel_size)
            })
        })
    })
}

pub fn fallbackfont(pixel_size: Option<LogicalLength>, scale_factor: ScaleFactor) -> VectorFont {
    let requested_pixel_size: PhysicalLength =
        (pixel_size.unwrap_or(super::DEFAULT_FONT_SIZE).cast() * scale_factor).cast();

    let fallback_font_id = FALLBACK_FONT_ID.with(|id| **id);

    sharedfontdb::FONT_DB
        .with(|fonts| {
            let fonts_borrowed = fonts.borrow();

            let fontdue_font = get_or_create_fontdue_font(&*fonts_borrowed, fallback_font_id);
            VectorFont::new(fallback_font_id, fontdue_font, requested_pixel_size)
        })
        .into()
}

pub fn register_font_from_memory(data: &'static [u8]) -> Result<(), Box<dyn std::error::Error>> {
    sharedfontdb::FONT_DB.with(|fonts| {
        fonts.borrow_mut().load_font_source(fontdb::Source::Binary(std::sync::Arc::new(data)))
    });
    Ok(())
}

pub fn register_font_from_path(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let requested_path = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    sharedfontdb::FONT_DB.with(|fonts| {
        for face_info in fonts.borrow().faces() {
            match &face_info.source {
                fontdb::Source::Binary(_) => {}
                fontdb::Source::File(loaded_path) | fontdb::Source::SharedFile(loaded_path, ..) => {
                    if *loaded_path == requested_path {
                        return Ok(());
                    }
                }
            }
        }

        fonts.borrow_mut().load_font_file(requested_path).map_err(|e| e.into())
    })
}
