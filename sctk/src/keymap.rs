// Borrowed from winit

pub fn keysym_to_key(keysym: u32) -> Key {
    use xkbcommon_dl::keysyms;
    Key::Named(match keysym {
        // TTY function keys
        keysyms::BackSpace => Named::Backspace,
        keysyms::Tab => Named::Tab,
        // keysyms::Linefeed => Named::Linefeed,
        keysyms::Clear => Named::Clear,
        keysyms::Return => Named::Enter,
        keysyms::Pause => Named::Pause,
        keysyms::Scroll_Lock => Named::ScrollLock,
        keysyms::Sys_Req => Named::PrintScreen,
        keysyms::Escape => Named::Escape,
        keysyms::Delete => Named::Delete,

        // IME keys
        keysyms::Multi_key => Named::Compose,
        keysyms::Codeinput => Named::CodeInput,
        keysyms::SingleCandidate => Named::SingleCandidate,
        keysyms::MultipleCandidate => Named::AllCandidates,
        keysyms::PreviousCandidate => Named::PreviousCandidate,

        // Japanese key
        keysyms::Kanji => Named::KanjiMode,
        keysyms::Muhenkan => Named::NonConvert,
        keysyms::Henkan_Mode => Named::Convert,
        keysyms::Romaji => Named::Romaji,
        keysyms::Hiragana => Named::Hiragana,
        keysyms::Hiragana_Katakana => Named::HiraganaKatakana,
        keysyms::Zenkaku => Named::Zenkaku,
        keysyms::Hankaku => Named::Hankaku,
        keysyms::Zenkaku_Hankaku => Named::ZenkakuHankaku,
        // keysyms::Touroku => Named::Touroku,
        // keysyms::Massyo => Named::Massyo,
        keysyms::Kana_Lock => Named::KanaMode,
        keysyms::Kana_Shift => Named::KanaMode,
        keysyms::Eisu_Shift => Named::Alphanumeric,
        keysyms::Eisu_toggle => Named::Alphanumeric,
        // NOTE: The next three items are aliases for values we've already mapped.
        // keysyms::Kanji_Bangou => Named::CodeInput,
        // keysyms::Zen_Koho => Named::AllCandidates,
        // keysyms::Mae_Koho => Named::PreviousCandidate,

        // Cursor control & motion
        keysyms::Home => Named::Home,
        keysyms::Left => Named::ArrowLeft,
        keysyms::Up => Named::ArrowUp,
        keysyms::Right => Named::ArrowRight,
        keysyms::Down => Named::ArrowDown,
        // keysyms::Prior => Named::PageUp,
        keysyms::Page_Up => Named::PageUp,
        // keysyms::Next => Named::PageDown,
        keysyms::Page_Down => Named::PageDown,
        keysyms::End => Named::End,
        // keysyms::Begin => Named::Begin,

        // Misc. functions
        keysyms::Select => Named::Select,
        keysyms::Print => Named::PrintScreen,
        keysyms::Execute => Named::Execute,
        keysyms::Insert => Named::Insert,
        keysyms::Undo => Named::Undo,
        keysyms::Redo => Named::Redo,
        keysyms::Menu => Named::ContextMenu,
        keysyms::Find => Named::Find,
        keysyms::Cancel => Named::Cancel,
        keysyms::Help => Named::Help,
        keysyms::Break => Named::Pause,
        keysyms::Mode_switch => Named::ModeChange,
        // keysyms::script_switch => Named::ModeChange,
        keysyms::Num_Lock => Named::NumLock,

        // Keypad keys
        // keysyms::KP_Space => return Key::Character(" "),
        keysyms::KP_Tab => Named::Tab,
        keysyms::KP_Enter => Named::Enter,
        keysyms::KP_F1 => Named::F1,
        keysyms::KP_F2 => Named::F2,
        keysyms::KP_F3 => Named::F3,
        keysyms::KP_F4 => Named::F4,
        keysyms::KP_Home => Named::Home,
        keysyms::KP_Left => Named::ArrowLeft,
        keysyms::KP_Up => Named::ArrowUp,
        keysyms::KP_Right => Named::ArrowRight,
        keysyms::KP_Down => Named::ArrowDown,
        // keysyms::KP_Prior => Named::PageUp,
        keysyms::KP_Page_Up => Named::PageUp,
        // keysyms::KP_Next => Named::PageDown,
        keysyms::KP_Page_Down => Named::PageDown,
        keysyms::KP_End => Named::End,
        // This is the key labeled "5" on the numpad when NumLock is off.
        // keysyms::KP_Begin => Named::Begin,
        keysyms::KP_Insert => Named::Insert,
        keysyms::KP_Delete => Named::Delete,
        // keysyms::KP_Equal => Named::Equal,
        // keysyms::KP_Multiply => Named::Multiply,
        // keysyms::KP_Add => Named::Add,
        // keysyms::KP_Separator => Named::Separator,
        // keysyms::KP_Subtract => Named::Subtract,
        // keysyms::KP_Decimal => Named::Decimal,
        // keysyms::KP_Divide => Named::Divide,

        // keysyms::KP_0 => return Key::Character("0"),
        // keysyms::KP_1 => return Key::Character("1"),
        // keysyms::KP_2 => return Key::Character("2"),
        // keysyms::KP_3 => return Key::Character("3"),
        // keysyms::KP_4 => return Key::Character("4"),
        // keysyms::KP_5 => return Key::Character("5"),
        // keysyms::KP_6 => return Key::Character("6"),
        // keysyms::KP_7 => return Key::Character("7"),
        // keysyms::KP_8 => return Key::Character("8"),
        // keysyms::KP_9 => return Key::Character("9"),

        // Function keys
        keysyms::F1 => Named::F1,
        keysyms::F2 => Named::F2,
        keysyms::F3 => Named::F3,
        keysyms::F4 => Named::F4,
        keysyms::F5 => Named::F5,
        keysyms::F6 => Named::F6,
        keysyms::F7 => Named::F7,
        keysyms::F8 => Named::F8,
        keysyms::F9 => Named::F9,
        keysyms::F10 => Named::F10,
        keysyms::F11 => Named::F11,
        keysyms::F12 => Named::F12,
        keysyms::F13 => Named::F13,
        keysyms::F14 => Named::F14,
        keysyms::F15 => Named::F15,
        keysyms::F16 => Named::F16,
        keysyms::F17 => Named::F17,
        keysyms::F18 => Named::F18,
        keysyms::F19 => Named::F19,
        keysyms::F20 => Named::F20,
        keysyms::F21 => Named::F21,
        keysyms::F22 => Named::F22,
        keysyms::F23 => Named::F23,
        keysyms::F24 => Named::F24,
        keysyms::F25 => Named::F25,
        keysyms::F26 => Named::F26,
        keysyms::F27 => Named::F27,
        keysyms::F28 => Named::F28,
        keysyms::F29 => Named::F29,
        keysyms::F30 => Named::F30,
        keysyms::F31 => Named::F31,
        keysyms::F32 => Named::F32,
        keysyms::F33 => Named::F33,
        keysyms::F34 => Named::F34,
        keysyms::F35 => Named::F35,

        // Modifiers
        keysyms::Shift_L => Named::Shift,
        keysyms::Shift_R => Named::Shift,
        keysyms::Control_L => Named::Control,
        keysyms::Control_R => Named::Control,
        keysyms::Caps_Lock => Named::CapsLock,
        // keysyms::Shift_Lock => Named::ShiftLock,

        // keysyms::Meta_L => Named::Meta,
        // keysyms::Meta_R => Named::Meta,
        keysyms::Alt_L => Named::Alt,
        keysyms::Alt_R => Named::Alt,
        keysyms::Super_L => Named::Super,
        keysyms::Super_R => Named::Super,
        keysyms::Hyper_L => Named::Hyper,
        keysyms::Hyper_R => Named::Hyper,

        // XKB function and modifier keys
        // keysyms::ISO_Lock => Named::IsoLock,
        // keysyms::ISO_Level2_Latch => Named::IsoLevel2Latch,
        keysyms::ISO_Level3_Shift => Named::AltGraph,
        keysyms::ISO_Level3_Latch => Named::AltGraph,
        keysyms::ISO_Level3_Lock => Named::AltGraph,
        // keysyms::ISO_Level5_Shift => Named::IsoLevel5Shift,
        // keysyms::ISO_Level5_Latch => Named::IsoLevel5Latch,
        // keysyms::ISO_Level5_Lock => Named::IsoLevel5Lock,
        // keysyms::ISO_Group_Shift => Named::IsoGroupShift,
        // keysyms::ISO_Group_Latch => Named::IsoGroupLatch,
        // keysyms::ISO_Group_Lock => Named::IsoGroupLock,
        keysyms::ISO_Next_Group => Named::GroupNext,
        // keysyms::ISO_Next_Group_Lock => Named::GroupNextLock,
        keysyms::ISO_Prev_Group => Named::GroupPrevious,
        // keysyms::ISO_Prev_Group_Lock => Named::GroupPreviousLock,
        keysyms::ISO_First_Group => Named::GroupFirst,
        // keysyms::ISO_First_Group_Lock => Named::GroupFirstLock,
        keysyms::ISO_Last_Group => Named::GroupLast,
        // keysyms::ISO_Last_Group_Lock => Named::GroupLastLock,
        //
        keysyms::ISO_Left_Tab => Named::Tab,
        // keysyms::ISO_Move_Line_Up => Named::IsoMoveLineUp,
        // keysyms::ISO_Move_Line_Down => Named::IsoMoveLineDown,
        // keysyms::ISO_Partial_Line_Up => Named::IsoPartialLineUp,
        // keysyms::ISO_Partial_Line_Down => Named::IsoPartialLineDown,
        // keysyms::ISO_Partial_Space_Left => Named::IsoPartialSpaceLeft,
        // keysyms::ISO_Partial_Space_Right => Named::IsoPartialSpaceRight,
        // keysyms::ISO_Set_Margin_Left => Named::IsoSetMarginLeft,
        // keysyms::ISO_Set_Margin_Right => Named::IsoSetMarginRight,
        // keysyms::ISO_Release_Margin_Left => Named::IsoReleaseMarginLeft,
        // keysyms::ISO_Release_Margin_Right => Named::IsoReleaseMarginRight,
        // keysyms::ISO_Release_Both_Margins => Named::IsoReleaseBothMargins,
        // keysyms::ISO_Fast_Cursor_Left => Named::IsoFastCursorLeft,
        // keysyms::ISO_Fast_Cursor_Right => Named::IsoFastCursorRight,
        // keysyms::ISO_Fast_Cursor_Up => Named::IsoFastCursorUp,
        // keysyms::ISO_Fast_Cursor_Down => Named::IsoFastCursorDown,
        // keysyms::ISO_Continuous_Underline => Named::IsoContinuousUnderline,
        // keysyms::ISO_Discontinuous_Underline => Named::IsoDiscontinuousUnderline,
        // keysyms::ISO_Emphasize => Named::IsoEmphasize,
        // keysyms::ISO_Center_Object => Named::IsoCenterObject,
        keysyms::ISO_Enter => Named::Enter,

        // dead_grave..dead_currency

        // dead_lowline..dead_longsolidusoverlay

        // dead_a..dead_capital_schwa

        // dead_greek

        // First_Virtual_Screen..Terminate_Server

        // AccessX_Enable..AudibleBell_Enable

        // Pointer_Left..Pointer_Drag5

        // Pointer_EnableKeys..Pointer_DfltBtnPrev

        // ch..C_H

        // 3270 terminal keys
        // keysyms::3270_Duplicate => Named::Duplicate,
        // keysyms::3270_FieldMark => Named::FieldMark,
        // keysyms::3270_Right2 => Named::Right2,
        // keysyms::3270_Left2 => Named::Left2,
        // keysyms::3270_BackTab => Named::BackTab,
        keysyms::_3270_EraseEOF => Named::EraseEof,
        // keysyms::3270_EraseInput => Named::EraseInput,
        // keysyms::3270_Reset => Named::Reset,
        // keysyms::3270_Quit => Named::Quit,
        // keysyms::3270_PA1 => Named::Pa1,
        // keysyms::3270_PA2 => Named::Pa2,
        // keysyms::3270_PA3 => Named::Pa3,
        // keysyms::3270_Test => Named::Test,
        keysyms::_3270_Attn => Named::Attn,
        // keysyms::3270_CursorBlink => Named::CursorBlink,
        // keysyms::3270_AltCursor => Named::AltCursor,
        // keysyms::3270_KeyClick => Named::KeyClick,
        // keysyms::3270_Jump => Named::Jump,
        // keysyms::3270_Ident => Named::Ident,
        // keysyms::3270_Rule => Named::Rule,
        // keysyms::3270_Copy => Named::Copy,
        keysyms::_3270_Play => Named::Play,
        // keysyms::3270_Setup => Named::Setup,
        // keysyms::3270_Record => Named::Record,
        // keysyms::3270_ChangeScreen => Named::ChangeScreen,
        // keysyms::3270_DeleteWord => Named::DeleteWord,
        keysyms::_3270_ExSelect => Named::ExSel,
        keysyms::_3270_CursorSelect => Named::CrSel,
        keysyms::_3270_PrintScreen => Named::PrintScreen,
        keysyms::_3270_Enter => Named::Enter,

        keysyms::space => Named::Space,
        // exclam..Sinh_kunddaliya

        // XFree86
        // keysyms::XF86_ModeLock => Named::ModeLock,

        // XFree86 - Backlight controls
        keysyms::XF86_MonBrightnessUp => Named::BrightnessUp,
        keysyms::XF86_MonBrightnessDown => Named::BrightnessDown,
        // keysyms::XF86_KbdLightOnOff => Named::LightOnOff,
        // keysyms::XF86_KbdBrightnessUp => Named::KeyboardBrightnessUp,
        // keysyms::XF86_KbdBrightnessDown => Named::KeyboardBrightnessDown,

        // XFree86 - "Internet"
        keysyms::XF86_Standby => Named::Standby,
        keysyms::XF86_AudioLowerVolume => Named::AudioVolumeDown,
        keysyms::XF86_AudioRaiseVolume => Named::AudioVolumeUp,
        keysyms::XF86_AudioPlay => Named::MediaPlay,
        keysyms::XF86_AudioStop => Named::MediaStop,
        keysyms::XF86_AudioPrev => Named::MediaTrackPrevious,
        keysyms::XF86_AudioNext => Named::MediaTrackNext,
        keysyms::XF86_HomePage => Named::BrowserHome,
        keysyms::XF86_Mail => Named::LaunchMail,
        // keysyms::XF86_Start => Named::Start,
        keysyms::XF86_Search => Named::BrowserSearch,
        keysyms::XF86_AudioRecord => Named::MediaRecord,

        // XFree86 - PDA
        keysyms::XF86_Calculator => Named::LaunchApplication2,
        // keysyms::XF86_Memo => Named::Memo,
        // keysyms::XF86_ToDoList => Named::ToDoList,
        keysyms::XF86_Calendar => Named::LaunchCalendar,
        keysyms::XF86_PowerDown => Named::Power,
        // keysyms::XF86_ContrastAdjust => Named::AdjustContrast,
        // keysyms::XF86_RockerUp => Named::RockerUp,
        // keysyms::XF86_RockerDown => Named::RockerDown,
        // keysyms::XF86_RockerEnter => Named::RockerEnter,

        // XFree86 - More "Internet"
        keysyms::XF86_Back => Named::BrowserBack,
        keysyms::XF86_Forward => Named::BrowserForward,
        // keysyms::XF86_Stop => Named::Stop,
        keysyms::XF86_Refresh => Named::BrowserRefresh,
        keysyms::XF86_PowerOff => Named::Power,
        keysyms::XF86_WakeUp => Named::WakeUp,
        keysyms::XF86_Eject => Named::Eject,
        keysyms::XF86_ScreenSaver => Named::LaunchScreenSaver,
        keysyms::XF86_WWW => Named::LaunchWebBrowser,
        keysyms::XF86_Sleep => Named::Standby,
        keysyms::XF86_Favorites => Named::BrowserFavorites,
        keysyms::XF86_AudioPause => Named::MediaPause,
        // keysyms::XF86_AudioMedia => Named::AudioMedia,
        keysyms::XF86_MyComputer => Named::LaunchApplication1,
        // keysyms::XF86_VendorHome => Named::VendorHome,
        // keysyms::XF86_LightBulb => Named::LightBulb,
        // keysyms::XF86_Shop => Named::BrowserShop,
        // keysyms::XF86_History => Named::BrowserHistory,
        // keysyms::XF86_OpenURL => Named::OpenUrl,
        // keysyms::XF86_AddFavorite => Named::AddFavorite,
        // keysyms::XF86_HotLinks => Named::HotLinks,
        // keysyms::XF86_BrightnessAdjust => Named::BrightnessAdjust,
        // keysyms::XF86_Finance => Named::BrowserFinance,
        // keysyms::XF86_Community => Named::BrowserCommunity,
        keysyms::XF86_AudioRewind => Named::MediaRewind,
        // keysyms::XF86_BackForward => Key::???,
        // XF86_Launch0..XF86_LaunchF

        // XF86_ApplicationLeft..XF86_CD
        keysyms::XF86_Calculater => Named::LaunchApplication2, // Nice typo, libxkbcommon :)
        // XF86_Clear
        keysyms::XF86_Close => Named::Close,
        keysyms::XF86_Copy => Named::Copy,
        keysyms::XF86_Cut => Named::Cut,
        // XF86_Display..XF86_Documents
        keysyms::XF86_Excel => Named::LaunchSpreadsheet,
        // XF86_Explorer..XF86iTouch
        keysyms::XF86_LogOff => Named::LogOff,
        // XF86_Market..XF86_MenuPB
        keysyms::XF86_MySites => Named::BrowserFavorites,
        keysyms::XF86_New => Named::New,
        // XF86_News..XF86_OfficeHome
        keysyms::XF86_Open => Named::Open,
        // XF86_Option
        keysyms::XF86_Paste => Named::Paste,
        keysyms::XF86_Phone => Named::LaunchPhone,
        // XF86_Q
        keysyms::XF86_Reply => Named::MailReply,
        keysyms::XF86_Reload => Named::BrowserRefresh,
        // XF86_RotateWindows..XF86_RotationKB
        keysyms::XF86_Save => Named::Save,
        // XF86_ScrollUp..XF86_ScrollClick
        keysyms::XF86_Send => Named::MailSend,
        keysyms::XF86_Spell => Named::SpellCheck,
        keysyms::XF86_SplitScreen => Named::SplitScreenToggle,
        // XF86_Support..XF86_User2KB
        keysyms::XF86_Video => Named::LaunchMediaPlayer,
        // XF86_WheelButton
        keysyms::XF86_Word => Named::LaunchWordProcessor,
        // XF86_Xfer
        keysyms::XF86_ZoomIn => Named::ZoomIn,
        keysyms::XF86_ZoomOut => Named::ZoomOut,

        // XF86_Away..XF86_Messenger
        keysyms::XF86_WebCam => Named::LaunchWebCam,
        keysyms::XF86_MailForward => Named::MailForward,
        // XF86_Pictures
        keysyms::XF86_Music => Named::LaunchMusicPlayer,

        // XF86_Battery..XF86_UWB
        //
        keysyms::XF86_AudioForward => Named::MediaFastForward,
        // XF86_AudioRepeat
        keysyms::XF86_AudioRandomPlay => Named::RandomToggle,
        keysyms::XF86_Subtitle => Named::Subtitle,
        keysyms::XF86_AudioCycleTrack => Named::MediaAudioTrack,
        // XF86_CycleAngle..XF86_Blue
        //
        keysyms::XF86_Suspend => Named::Standby,
        keysyms::XF86_Hibernate => Named::Hibernate,
        // XF86_TouchpadToggle..XF86_TouchpadOff
        //
        keysyms::XF86_AudioMute => Named::AudioVolumeMute,

        // XF86_Switch_VT_1..XF86_Switch_VT_12

        // XF86_Ungrab..XF86_ClearGrab
        keysyms::XF86_Next_VMode => Named::VideoModeNext,
        // keysyms::XF86_Prev_VMode => Named::VideoModePrevious,
        // XF86_LogWindowTree..XF86_LogGrabInfo

        // SunFA_Grave..SunFA_Cedilla

        // keysyms::SunF36 => Named::F36 | Named::F11,
        // keysyms::SunF37 => Named::F37 | Named::F12,

        // keysyms::SunSys_Req => Named::PrintScreen,
        // The next couple of xkb (until SunStop) are already handled.
        // SunPrint_Screen..SunPageDown

        // SunUndo..SunFront
        keysyms::SUN_Copy => Named::Copy,
        keysyms::SUN_Open => Named::Open,
        keysyms::SUN_Paste => Named::Paste,
        keysyms::SUN_Cut => Named::Cut,

        // SunPowerSwitch
        keysyms::SUN_AudioLowerVolume => Named::AudioVolumeDown,
        keysyms::SUN_AudioMute => Named::AudioVolumeMute,
        keysyms::SUN_AudioRaiseVolume => Named::AudioVolumeUp,
        // SUN_VideoDegauss
        keysyms::SUN_VideoLowerBrightness => Named::BrightnessDown,
        keysyms::SUN_VideoRaiseBrightness => Named::BrightnessUp,
        // SunPowerSwitchShift
        //
        _ => return Key::Unidentified,
    })
}

use iced_runtime::keyboard::{key::Named, Key, Location};

pub fn keysym_location(keysym: u32) -> Location {
    use xkbcommon_dl::keysyms;
    match keysym {
        xkeysym::key::Shift_L
        | keysyms::Control_L
        | keysyms::Meta_L
        | keysyms::Alt_L
        | keysyms::Super_L
        | keysyms::Hyper_L => Location::Left,
        keysyms::Shift_R
        | keysyms::Control_R
        | keysyms::Meta_R
        | keysyms::Alt_R
        | keysyms::Super_R
        | keysyms::Hyper_R => Location::Right,
        keysyms::KP_0
        | keysyms::KP_1
        | keysyms::KP_2
        | keysyms::KP_3
        | keysyms::KP_4
        | keysyms::KP_5
        | keysyms::KP_6
        | keysyms::KP_7
        | keysyms::KP_8
        | keysyms::KP_9
        | keysyms::KP_Space
        | keysyms::KP_Tab
        | keysyms::KP_Enter
        | keysyms::KP_F1
        | keysyms::KP_F2
        | keysyms::KP_F3
        | keysyms::KP_F4
        | keysyms::KP_Home
        | keysyms::KP_Left
        | keysyms::KP_Up
        | keysyms::KP_Right
        | keysyms::KP_Down
        | keysyms::KP_Page_Up
        | keysyms::KP_Page_Down
        | keysyms::KP_End
        | keysyms::KP_Begin
        | keysyms::KP_Insert
        | keysyms::KP_Delete
        | keysyms::KP_Equal
        | keysyms::KP_Multiply
        | keysyms::KP_Add
        | keysyms::KP_Separator
        | keysyms::KP_Subtract
        | keysyms::KP_Decimal
        | keysyms::KP_Divide => Location::Numpad,
        _ => Location::Standard,
    }
}
