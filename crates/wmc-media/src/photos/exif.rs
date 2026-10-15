use std::path::Path;
pub fn photo_time(path: &Path) -> Option<i64> {
    use chrono::{Local, NaiveDateTime, TimeZone};
    let taken = (|| {
        let file = std::fs::File::open(path).ok()?;
        let exif = exif::Reader::new()
            .read_from_container(&mut std::io::BufReader::new(file))
            .ok()?;
        let field = exif
            .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
            .or_else(|| exif.get_field(exif::Tag::DateTimeDigitized, exif::In::PRIMARY))
            .or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
        let exif::Value::Ascii(values) = &field.value else {
            return None;
        };
        let text = std::str::from_utf8(values.first()?)
            .ok()?
            .trim_end_matches('\0');
        let date = NaiveDateTime::parse_from_str(text, "%Y:%m:%d %H:%M:%S").ok()?;
        if let Some(offset) = exif.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY)
            && let exif::Value::Ascii(values) = &offset.value
            && let Some(value) = values.first()
            && let Ok(offset) = std::str::from_utf8(value)
            && let Ok(date) = chrono::DateTime::parse_from_str(
                &format!("{text}{}", offset.trim_end_matches('\0')),
                "%Y:%m:%d %H:%M:%S%:z",
            )
        {
            return Some(date.timestamp());
        }
        Local
            .from_local_datetime(&date)
            .earliest()
            .map(|date| date.timestamp())
    })();
    taken.or_else(|| {
        std::fs::metadata(path)
            .ok()?
            .modified()
            .ok()
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time).timestamp())
    })
}
