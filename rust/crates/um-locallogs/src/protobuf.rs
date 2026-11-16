//! 아주 작은 raw protobuf 걷기(외부 크레이트 없음). 스키마 없이 필드 번호로만 접근한다.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wire<'a> {
    Varint(u64),
    Fixed64(u64),
    Len(&'a [u8]),
    Fixed32(u32),
}

fn varint(buf: &[u8], pos: &mut usize) -> Option<u64> {
    let mut v = 0u64;
    for shift in (0..70).step_by(7) {
        let b = *buf.get(*pos)?;
        *pos += 1;
        if shift < 64 {
            v |= u64::from(b & 0x7f) << shift;
        }
        if b & 0x80 == 0 {
            return Some(v);
        }
    }
    None
}

/// 최상위 필드를 순서대로 돌려준다. 형식이 깨지면 거기서 멈춘다.
pub fn fields(buf: &[u8]) -> impl Iterator<Item = (u32, Wire<'_>)> {
    let mut pos = 0usize;
    std::iter::from_fn(move || {
        if pos >= buf.len() {
            return None;
        }
        let tag = varint(buf, &mut pos)?;
        let num = u32::try_from(tag >> 3).ok()?;
        let wire = match tag & 7 {
            0 => Wire::Varint(varint(buf, &mut pos)?),
            1 => {
                let b = buf.get(pos..pos.checked_add(8)?)?;
                pos += 8;
                Wire::Fixed64(u64::from_le_bytes(b.try_into().ok()?))
            }
            2 => {
                let len = usize::try_from(varint(buf, &mut pos)?).ok()?;
                let end = pos.checked_add(len)?;
                let b = buf.get(pos..end)?;
                pos = end;
                Wire::Len(b)
            }
            5 => {
                let b = buf.get(pos..pos.checked_add(4)?)?;
                pos += 4;
                Wire::Fixed32(u32::from_le_bytes(b.try_into().ok()?))
            }
            _ => return None,
        };
        Some((num, wire))
    })
}

/// 필드 번호 경로(예: `[1, 19]`)를 따라 내려가 마지막 필드의 첫 값을 찾는다.
pub fn find_path<'a>(buf: &'a [u8], path: &[u32]) -> Option<Wire<'a>> {
    let (first, rest) = path.split_first()?;
    let (_, w) = fields(buf).find(|(n, _)| n == first)?;
    if rest.is_empty() {
        return Some(w);
    }
    match w {
        Wire::Len(inner) => find_path(inner, rest),
        _ => None,
    }
}

pub fn varint_at(buf: &[u8], path: &[u32]) -> Option<u64> {
    match find_path(buf, path)? {
        Wire::Varint(v) => Some(v),
        _ => None,
    }
}

pub fn string_at(buf: &[u8], path: &[u32]) -> Option<String> {
    match find_path(buf, path)? {
        Wire::Len(b) => std::str::from_utf8(b).ok().map(str::to_string),
        _ => None,
    }
}

/// 테스트용 인코더(필드를 하나씩 이어 붙인다).
pub mod enc {
    pub fn varint(mut v: u64, out: &mut Vec<u8>) {
        while v >= 0x80 {
            out.push((v as u8 & 0x7f) | 0x80);
            v >>= 7;
        }
        out.push(v as u8);
    }
    pub fn field_varint(num: u32, v: u64, out: &mut Vec<u8>) {
        varint(u64::from(num) << 3, out);
        varint(v, out);
    }
    pub fn field_len(num: u32, data: &[u8], out: &mut Vec<u8>) {
        varint((u64::from(num) << 3) | 2, out);
        varint(data.len() as u64, out);
        out.extend_from_slice(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_nested_path() {
        let mut usage = Vec::new();
        enc::field_varint(1, 1318, &mut usage);
        enc::field_varint(2, 300, &mut usage); // 2바이트 varint
        let mut inner = Vec::new();
        enc::field_len(4, &usage, &mut inner);
        enc::field_len(19, b"model-x", &mut inner);
        let mut root = Vec::new();
        enc::field_len(1, &inner, &mut root);
        enc::field_varint(7, 1, &mut root);

        assert_eq!(string_at(&root, &[1, 19]).as_deref(), Some("model-x"));
        assert_eq!(varint_at(&root, &[1, 4, 2]), Some(300));
        assert_eq!(varint_at(&root, &[1, 4, 1]), Some(1318));
        assert_eq!(varint_at(&root, &[1, 4, 3]), None);
        assert_eq!(varint_at(&root, &[7]), Some(1));
    }

    #[test]
    fn truncated_input_stops_without_panic() {
        let mut root = Vec::new();
        enc::field_len(1, b"abcdef", &mut root);
        root.truncate(4);
        assert_eq!(fields(&root).count(), 0);
        assert_eq!(fields(&[0x08]).count(), 0);
    }
}
