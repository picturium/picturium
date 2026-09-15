use anyhow::Result;
use picturium_libvips::{IccTransformOptions, VipsImage, VipsOperations};

/// Convert embedded profiles to sRGB, skipping recognized sRGB descriptions.
pub fn process(image: VipsImage) -> Result<VipsImage> {
    let Ok(profile) = image.get_blob("icc-profile-data") else {
        return Ok(image);
    };

    if is_srgb(&profile) {
        return Ok(image);
    }

    let options = IccTransformOptions {
        embedded: true,
        ..Default::default()
    };

    match image.clone().icc_transform("srgb", Some(options)) {
        Ok(image) => Ok(image),
        Err(error) => {
            tracing::warn!(?error, "Failed to convert image to sRGB, keeping it as is");
            Ok(image)
        }
    }
}

// Trust known profile names for speed; unknown descriptions use ICC conversion.
fn is_srgb(profile: &[u8]) -> bool {
    profile.get(16..20) == Some(b"RGB ")
        && tag(profile, b"desc")
            .and_then(is_srgb_description)
            .unwrap_or(false)
}

fn is_srgb_description(body: &[u8]) -> Option<bool> {
    match body.get(..4)? {
        b"desc" => {
            let length = read_u32(body, 8)?;
            let name = std::str::from_utf8(body.get(12..)?.get(..length)?).ok()?;
            
            Some(is_srgb_name(name))
        }
        b"mluc" => {
            let count = read_u32(body, 8)?;
            let record_size = read_u32(body, 12)?;
            
            if record_size != 12 {
                return None;
            }

            let records = body.get(16..)?.get(..count.checked_mul(record_size)?)?;
            
            for record in records.chunks_exact(record_size) {
                let length = read_u32(record, 4)?;
                let offset = read_u32(record, 8)?;
                let text = body.get(offset..)?.get(..length)?;
                
                if text.len() % 2 != 0 {
                    return None;
                }

                let units = text
                    .chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
                
                let name = char::decode_utf16(units)
                    .collect::<std::result::Result<String, _>>()
                    .ok()?;
                
                if is_srgb_name(&name) {
                    return Some(true);
                }
            }

            Some(false)
        }
        _ => None,
    }
}

fn is_srgb_name(name: &str) -> bool {
    let name = name.trim_end_matches('\0').trim();
    
    ["sRGB", "sRGB IEC61966-2.1", "sRGB IEC61966-2-1", "sRGB built-in"]
        .iter()
        .any(|known| name.eq_ignore_ascii_case(known))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<usize> {
    Some(u32::from_be_bytes(bytes.get(offset..)?.get(..4)?.try_into().ok()?) as usize)
}

fn tag<'a>(profile: &'a [u8], signature: &[u8; 4]) -> Option<&'a [u8]> {
    let count = read_u32(profile, 128)?;
    let records = profile.get(132..)?.get(..count.checked_mul(12)?)?;

    for record in records.chunks_exact(12) {
        if &record[..4] == signature {
            let offset = read_u32(record, 4)?;
            let length = read_u32(record, 8)?;
            
            return profile.get(offset..)?.get(..length);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(body: &[u8]) -> Vec<u8> {
        let mut profile = vec![0; 132];
        profile[16..20].copy_from_slice(b"RGB ");
        profile[128..132].copy_from_slice(&1u32.to_be_bytes());
        profile.extend_from_slice(b"desc");
        profile.extend_from_slice(&144u32.to_be_bytes());
        profile.extend_from_slice(&(body.len() as u32).to_be_bytes());
        profile.extend_from_slice(body);
        profile
    }

    fn desc(name: &str) -> Vec<u8> {
        let mut body = b"desc\0\0\0\0".to_vec();
        body.extend_from_slice(&((name.len() + 1) as u32).to_be_bytes());
        body.extend_from_slice(name.as_bytes());
        body.push(0);
        profile(&body)
    }

    fn mluc(names: &[&str]) -> Vec<u8> {
        let mut body = b"mluc\0\0\0\0".to_vec();
        body.extend_from_slice(&(names.len() as u32).to_be_bytes());
        body.extend_from_slice(&12u32.to_be_bytes());
        let mut text = Vec::new();
        for name in names {
            let encoded: Vec<u8> = name.encode_utf16().flat_map(u16::to_be_bytes).collect();
            body.extend_from_slice(b"enUS");
            body.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
            body.extend_from_slice(&((16 + names.len() * 12 + text.len()) as u32).to_be_bytes());
            text.extend_from_slice(&encoded);
        }
        body.extend_from_slice(&text);
        profile(&body)
    }

    #[test]
    fn recognises_known_descriptions_in_both_encodings() {
        for name in [
            "sRGB",
            "sRGB IEC61966-2.1",
            "sRGB IEC61966-2-1",
            "sRGB built-in",
            " SRgb ",
        ] {
            assert!(is_srgb(&desc(name)), "{name}");
            assert!(is_srgb(&mluc(&[name])), "{name}");
        }
        assert!(is_srgb(&mluc(&["Profil couleur", "sRGB"])));
        for name in [
            "",
            "Display P3",
            "Adobe RGB (1998)",
            "linear sRGB",
            "e-sRGB",
            "scRGB",
            "sRGB custom",
        ] {
            assert!(!is_srgb(&desc(name)), "{name}");
            assert!(!is_srgb(&mluc(&[name])), "{name}");
        }
    }

    #[test]
    fn missing_malformed_and_non_rgb_profiles_do_not_skip_conversion() {
        for full in [desc("sRGB"), mluc(&["sRGB"])] {
            for length in 0..full.len() {
                assert!(!is_srgb(&full[..length]));
            }
            let mut non_rgb = full.clone();
            non_rgb[16..20].copy_from_slice(b"CMYK");
            assert!(!is_srgb(&non_rgb));
            let mut bad_offset = full;
            bad_offset[136..140].copy_from_slice(&u32::MAX.to_be_bytes());
            assert!(!is_srgb(&bad_offset));
        }
        assert!(!is_srgb(&profile(b"unknown")));

        let mut bad_count = desc("sRGB");
        bad_count[128..132].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(!is_srgb(&bad_count));

        let mut invalid_utf16 = mluc(&["sRGB"]);
        invalid_utf16[172..174].copy_from_slice(&0xD800u16.to_be_bytes());
        assert!(!is_srgb(&invalid_utf16));

        let mut odd_utf16 = mluc(&["sRGB"]);
        odd_utf16[164..168].copy_from_slice(&7u32.to_be_bytes());
        assert!(!is_srgb(&odd_utf16));
        let mut bad_record_size = mluc(&["sRGB"]);
        bad_record_size[156..160].copy_from_slice(&0u32.to_be_bytes());
        assert!(!is_srgb(&bad_record_size));
    }
}
