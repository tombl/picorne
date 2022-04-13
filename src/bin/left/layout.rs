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

// docs: https://github.com/TeXitoi/keyberon/pull/54
pub const LAYERS: Layers<12, 4, 3, CustomAction> = layout! {
    {
        [ Escape Q W F P B   J L U Y I BSpace ]
        [ LShift A R S T G   K N E I O Enter  ]
        [ LCtrl  Z X C D V   M H , . / RShift ]
        [ n n n LGui (1) Space   Space (2) LAlt n n n ]
    }
    {
        [ t ! @ # $ %   ^ & * '(' ')' Delete ]
        [ t CapsLock Insert Home + ?   Left Down Up Right ; '"' ]
        [ t n n n n n   < > '[' ']' '\\' [RShift Grave] ]
        [ n n n t t t   t t t n n n ]
    }
    {
        [ t 1 2 3 4 5   6 7 8 9 0 '_' ]
        [ t F11 F12 End PgUp PgDown   - = '{' '}' {MultipleActions(&[KeyCode(KeyCode::RShift), KeyCode(KeyCode::SColon)])} '\'' ]
        [ t F1 F2 F3 F4 F5   F6 F7 F8 F9 F10 '`' ]
        [ n n n t t t   t t t n n n ]
    }
};
