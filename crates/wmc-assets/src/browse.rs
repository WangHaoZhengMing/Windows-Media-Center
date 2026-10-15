// AUDIO.ALBUMBOX.NOCOVER is a wide box skin, not a cover image.
pub const ALBUM_PLACEHOLDER: &str = "wmc/browse/AUDIO.NOCOVER.5.LARGE.PNG";
pub const VIDEO_PLACEHOLDER: &str = "wmc/browse/MOVIE.ICON.DEFAULT.PNG";
pub const PLAY_COMMAND: &str = "wmc/browse/BROWSE.CMD.PLAY.DORMANT.PNG";
pub const SEARCH_COMMAND: &str = "wmc/browse/BROWSE.CMD.SEARCH.DORMANT.PNG";
pub const REFRESH_COMMAND: &str = "wmc/icons/STARTMENU.QUICKLINK.SYNC.FOCUS.PNG";
pub const PIVOT_LEFT: &str = "wmc/browse/PIVOT.ARROW.LEFT.NOFOCUS.PNG";
pub const PIVOT_RIGHT: &str = "wmc/browse/PIVOT.ARROW.RIGHT.NOFOCUS.PNG";
pub const COMMAND_BUTTON: &str = "wmc/browse/COMMON.BUTTON.LEFT.FOCUS.PNG";
pub const COMMAND_BUTTON_IDLE: &str = "wmc/browse/COMMON.BUTTON.DORMANT.FULL.PNG";
pub const CHECKED: &str = "wmc/browse/FIRSTRUN.CHECKBOX.CHECKED.NOFOCUS.PNG";
pub const UNCHECKED: &str = "wmc/browse/FIRSTRUN.CHECKBOX.UNCHECKED.NOFOCUS.PNG";

#[cfg(test)]
mod tests {
    #[test]
    fn default_album_is_an_embedded_square_cover_not_a_box_strip() {
        let bytes = crate::load(super::ALBUM_PLACEHOLDER).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert_eq!(width, height);
        assert!(width >= 256);
    }
}
