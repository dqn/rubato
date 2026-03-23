fn pm_parse_int(s: &str) -> i32 {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    cleaned.parse::<i32>().unwrap_or(0)
}

fn pm_parse_int_radix(s: &str, radix: i32) -> i32 {
    if radix == 36 {
        if s.len() < 2 {
            return -1;
        }
        let mut result = 0_i32;
        let c1 = s.as_bytes()[0] as char;
        if c1.is_ascii_digit() {
            result = ((c1 as i32) - ('0' as i32)) * 36;
        } else if c1.is_ascii_lowercase() {
            result = (((c1 as i32) - ('a' as i32)) + 10) * 36;
        } else if c1.is_ascii_uppercase() {
            result = (((c1 as i32) - ('A' as i32)) + 10) * 36;
        }
        let c2 = s.as_bytes()[1] as char;
        if c2.is_ascii_digit() {
            result += (c2 as i32) - ('0' as i32);
        } else if c2.is_ascii_lowercase() {
            result += ((c2 as i32) - ('a' as i32)) + 10;
        } else if c2.is_ascii_uppercase() {
            result += ((c2 as i32) - ('A' as i32)) + 10;
        }
        return result;
    }
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit() || *c == '-')
        .collect();
    i32::from_str_radix(&cleaned, radix as u32).unwrap_or(-1)
}

fn pm_parse_str(s: &[&str]) -> Vec<String> {
    let mut list = Vec::new();
    for &item in s {
        if !item.is_empty() {
            if item.starts_with('/') {
                break;
            } else if let Some(pos) = item.find("//") {
                list.push(item[..pos].to_string());
                break;
            } else {
                list.push(item.to_string());
            }
        }
    }
    list
}

fn transparent_processing(
    tex: Option<Texture>,
    index: usize,
    flag: &mut [bool; 8],
) -> Option<Texture> {
    // Transparent processing: bottom-right 1 pixel is transparent color
    // SelectCG icons are not made transparent
    let tex = tex?;
    if flag[index] {
        return Some(tex);
    }

    let w = tex.width;
    let h = tex.height;
    if w <= 0 || h <= 0 {
        flag[index] = true;
        return Some(tex);
    }

    // Access pixel data from rgba_data
    let rgba_data = match tex.rgba_data.as_ref() {
        Some(data) => data,
        None => {
            flag[index] = true;
            return Some(tex);
        }
    };

    // Get transparent color from bottom-right pixel
    let br_idx = ((h as usize - 1) * w as usize + (w as usize - 1)) * 4;
    if br_idx + 3 >= rgba_data.len() {
        flag[index] = true;
        return Some(tex);
    }
    let tr = rgba_data[br_idx];
    let tg = rgba_data[br_idx + 1];
    let tb = rgba_data[br_idx + 2];
    let ta = rgba_data[br_idx + 3];

    // Create new pixmap with transparent color removed
    let mut new_data = vec![0u8; w as usize * h as usize * 4];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let idx = (y * w as usize + x) * 4;
            if idx + 3 < rgba_data.len() {
                let pr = rgba_data[idx];
                let pg = rgba_data[idx + 1];
                let pb = rgba_data[idx + 2];
                let pa = rgba_data[idx + 3];
                if pr != tr || pg != tg || pb != tb || pa != ta {
                    new_data[idx] = pr;
                    new_data[idx + 1] = pg;
                    new_data[idx + 2] = pb;
                    new_data[idx + 3] = pa;
                }
                // else: leave as 0,0,0,0 (transparent)
            }
        }
    }

    flag[index] = true;

    Some(Texture {
        width: w,
        height: h,
        disposed: false,
        path: tex.path.clone(),
        rgba_data: Some(Arc::new(new_data)),
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
        pixmap_id: tex.pixmap_id,
    })
}

