//! SF Symbol은 macOS 시스템에서만 그린다. PNG는 파일이나 앱 번들에 저장하지 않는다.

#[tauri::command]
pub fn ui_symbol(
    name: String,
    size: f64,
    weight: Option<String>,
    color: Option<String>,
) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        mac::symbol(name, size, weight, color)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (name, size, weight, color);
        None
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use objc2::{AnyThread, MainThreadMarker, rc::autoreleasepool};
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSCalibratedRGBColorSpace, NSColor,
        NSCompositingOperation, NSFontWeightBlack, NSFontWeightBold, NSFontWeightHeavy,
        NSFontWeightLight, NSFontWeightMedium, NSFontWeightRegular, NSFontWeightSemibold,
        NSFontWeightThin, NSFontWeightUltraLight, NSGraphicsContext, NSImage,
        NSImageSymbolConfiguration, NSRectFillUsingOperation,
    };
    use objc2_foundation::{
        NSDataBase64EncodingOptions, NSDictionary, NSPoint, NSRect, NSSize, NSString,
    };

    type Key = (String, u64, String, String);
    thread_local! {
        // 현재 UI의 작은 심볼은 수 KB 단위다. 임의 인자가 누적되지 않도록 총량도 제한한다.
        static CACHE: RefCell<HashMap<Key, String>> = RefCell::new(HashMap::new());
    }

    pub fn symbol(
        name: String,
        size: f64,
        weight: Option<String>,
        color: Option<String>,
    ) -> Option<String> {
        // 동기 Tauri 명령은 메인 스레드에서 실행된다. AppKit 객체는 그 안에서만 사용한다.
        let _mtm = MainThreadMarker::new()?;
        if name.is_empty()
            || name.len() > 128
            || !size.is_finite()
            || !(1.0..=128.0).contains(&size)
        {
            return None;
        }
        let weight = weight.unwrap_or_else(|| "regular".into());
        let color = color
            .unwrap_or_else(|| "#000000".into())
            .to_ascii_lowercase();
        let key = (name.clone(), size.to_bits(), weight.clone(), color.clone());
        if let Some(data) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
            return Some(data);
        }
        let data = autoreleasepool(|_| render(&name, size, &weight, &color))?;
        CACHE.with(|c| {
            let mut cache = c.borrow_mut();
            // 큰 심볼도 응답하되 64 KiB를 넘는 결과는 보관하지 않는다.
            if data.len() <= 64 * 1024 {
                if cache
                    .iter()
                    .map(|(k, v)| k.0.len() + k.2.len() + k.3.len() + v.len())
                    .sum::<usize>()
                    + data.len()
                    + 256
                    > 64 * 1024
                {
                    cache.clear();
                }
                cache.insert(key, data.clone());
            }
        });
        Some(data)
    }

    fn render(name: &str, size: f64, weight: &str, color: &str) -> Option<String> {
        // AppKit이 공개한 글꼴 굵기 상수만 읽는다.
        let weight = unsafe {
            match weight {
                "ultralight" => NSFontWeightUltraLight,
                "thin" => NSFontWeightThin,
                "light" => NSFontWeightLight,
                "regular" => NSFontWeightRegular,
                "medium" => NSFontWeightMedium,
                "semibold" => NSFontWeightSemibold,
                "bold" => NSFontWeightBold,
                "heavy" => NSFontWeightHeavy,
                "black" => NSFontWeightBlack,
                _ => return None,
            }
        };
        // 색 인자는 #RRGGBB 또는 #RRGGBBAA다. 프런트엔드는 기본 불투명 마스크를 currentColor로 칠한다.
        let hex = color.strip_prefix('#')?;
        if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .ok()
                .map(|v| v as f64 / 255.0)
        };
        let tint = NSColor::colorWithSRGBRed_green_blue_alpha(
            channel(0)?,
            channel(2)?,
            channel(4)?,
            if hex.len() == 8 { channel(6)? } else { 1.0 },
        );
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &NSString::from_str(name),
            None,
        )?;
        let config = NSImageSymbolConfiguration::configurationWithPointSize_weight(size, weight)
            .configurationByApplyingConfiguration(
                &NSImageSymbolConfiguration::configurationPreferringMonochrome(),
            );
        let image = image.imageWithSymbolConfiguration(&config)?;
        let pixels = (size * 2.0).ceil() as isize;
        // null 평면은 AppKit이 RGBA 버퍼를 직접 할당하게 한다. 해상도는 표시 크기의 2배다.
        let bitmap = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(), std::ptr::null_mut(), pixels, pixels, 8, 4, true, false, NSCalibratedRGBColorSpace, 0, 0,
            )
        }?;
        let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;
        let extent = pixels as f64;
        let original = image.size();
        if original.width <= 0.0 || original.height <= 0.0 {
            return None;
        }
        let scale = extent / original.width.max(original.height);
        let rect = NSRect::new(
            NSPoint::new(
                (extent - original.width * scale) / 2.0,
                (extent - original.height * scale) / 2.0,
            ),
            NSSize::new(original.width * scale, original.height * scale),
        );
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&context));
        let bounds = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(extent, extent));
        NSColor::clearColor().set();
        NSRectFillUsingOperation(bounds, NSCompositingOperation::Copy);
        image.drawInRect_fromRect_operation_fraction(
            rect,
            NSRect::ZERO,
            NSCompositingOperation::SourceOver,
            1.0,
        );
        tint.set();
        NSRectFillUsingOperation(bounds, NSCompositingOperation::SourceIn);
        NSGraphicsContext::restoreGraphicsState_class();
        // 빈 속성 사전은 PNG 표현에 별도 옵션을 전달하지 않는다.
        let png = unsafe {
            bitmap.representationUsingType_properties(
                NSBitmapImageFileType::PNG,
                &NSDictionary::new(),
            )
        }?;
        Some(format!(
            "data:image/png;base64,{}",
            png.base64EncodedStringWithOptions(NSDataBase64EncodingOptions::empty())
        ))
    }
}
