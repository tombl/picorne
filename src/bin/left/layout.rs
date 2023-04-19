#![allow(unused_imports)]
#![allow(clippy::enum_glob_use)]

use keyberon::{
    action::Action::*,
    key_code::KeyCode,
    layout::{layout, Layers},
};
pub enum CustomAction {
    Reset,
    Bootsel,
}
use CustomAction::*;
type Action = keyberon::action::Action<CustomAction>;

// docs: https://github.com/TeXitoi/keyberon/pull/54

const SHIFT_TAB: Action = MultipleKeyCodes(&[KeyCode::LShift, KeyCode::Tab].as_slice());
const QWERTY: Action = DefaultLayer(4);
const COLEMAK_DK: Action = DefaultLayer(0);

pub const LAYERS: Layers<12, 4, 5, CustomAction> = layout! {
    {
        [ Escape Q W F P B   J L U Y ; Delete ]
        [   LAlt A R S T G   M N E I O '\''   ]
        [ LShift Z X C D V   K H , . / Enter  ]
        [ n n n LCtrl (1) Space   BSpace (2) RGui  n n n ]
    }
    {
        [ t ! @ # $ %   ^ & * '(' ')' t ]
        [ t '`' ~ - '_' +   = '{' '}' '[' ']' '\\' ]
        [ t t t t t t   t t t t t t ]
        [ n n n t t t   t (3) t n n n ]
    }
    {
        [ t MediaCalc 7 8 9 -   PgUp Home End t t t ]
        [ t         * 4 5 6 +   PgDown Left Down Up Right t ]
        [ t         0 1 2 3 =   t Tab {SHIFT_TAB} t t t ]
        [ n       n n t (3) t   t t t n n n ]
    }
    {
        [ {Custom(Bootsel)} t t t t t   t MediaMute MediaVolDown MediaVolUp t t ]
        [ CapsLock PScreen t t t t   t MediaPlayPause MediaPreviousSong MediaNextSong t t ]
        [ t {COLEMAK_DK} {QWERTY} t t t   t t t t t t ]
        [ n n n t t t   t t t n n n ]
    }
    {
        [ Escape Q W E R T   Y U I O P Delete ]
        [   LAlt A S D F G   H J K L ; '\''   ]
        [ LShift Z X C V B   N M , . / Enter  ]
        [ n n n LCtrl (1) Space   BSpace (2) RGui  n n n ]
    }
};
