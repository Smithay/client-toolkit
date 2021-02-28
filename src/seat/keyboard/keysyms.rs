//
// This file was auto-generated using the update-keysyms.sh script.
//

#![allow(missing_docs, non_upper_case_globals, unused_parens, clippy::all)]

/***********************************************************
Copyright 1987, 1994, 1998  The Open Group

Permission to use, copy, modify, distribute, and sell this software and its
documentation for any purpose is hereby granted without fee, provided that
the above copyright notice appear in all copies and that both that
copyright notice and this permission notice appear in supporting
documentation.

The above copyright notice and this permission notice shall be included
in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE OPEN GROUP BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of The Open Group shall
not be used in advertising or otherwise to promote the sale, use or
other dealings in this Software without prior written authorization
from The Open Group.


Copyright 1987 by Digital Equipment Corporation, Maynard, Massachusetts

                        All Rights Reserved

Permission to use, copy, modify, and distribute this software and its
documentation for any purpose and without fee is hereby granted,
provided that the above copyright notice appear in all copies and that
both that copyright notice and this permission notice appear in
supporting documentation, and that the name of Digital not be
used in advertising or publicity pertaining to distribution of the
software without specific, written prior permission.

DIGITAL DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE, INCLUDING
ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS, IN NO EVENT SHALL
DIGITAL BE LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR
ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS
SOFTWARE.

******************************************************************/

/*
 * The "X11 Window System Protocol" standard defines in Appendix A the
 * keysym codes. These 29-bit integer values identify characters or
 * functions associated with each key (e.g., via the visible
 * engraving) of a keyboard layout. This file assigns mnemonic macro
 * names for these keysyms.
 *
 * This file is also compiled (by src/util/makekeys.c in libX11) into
 * hash tables that can be accessed with X11 library functions such as
 * XStringToKeysym() and XKeysymToString().
 *
 * Where a keysym corresponds one-to-one to an ISO 10646 / Unicode
 * character, this is noted in a comment that provides both the U+xxxx
 * Unicode position, as well as the official Unicode name of the
 * character.
 *
 * Where the correspondence is either not one-to-one or semantically
 * unclear, the Unicode position and name are enclosed in
 * parentheses. Such legacy keysyms should be considered deprecated
 * and are not recommended for use in future keyboard mappings.
 *
 * For any future extension of the keysyms with characters already
 * found in ISO 10646 / Unicode, the following algorithm shall be
 * used. The new keysym code position will simply be the character's
 * Unicode number plus :u32 = 0x01000000;. The keysym values in the range
 * :u32 = 0x01000100; to :u32 = 0x0110ffff; are reserved to represent Unicode
 * characters in the range U+0100 to U+10FFFF.
 *
 * While most newer Unicode-based X11 clients do already accept
 * Unicode-mapped keysyms in the range :u32 = 0x01000100; to :u32 = 0x0110ffff;, it
 * will remain necessary for clients -- in the interest of
 * compatibility with existing servers -- to also understand the
 * existing legacy keysym values in the range :u32 = 0x0100; to :u32 = 0x20ff;.
 *
 * Where several mnemonic names are defined for the same keysym in this
 * file, all but the first one listed should be considered deprecated.
 *
 * Mnemonic names for keysyms are defined in this file with lines
 * that match one of these Perl regular expressions:
 *
 *    /^\pub const XKB_KEY_([a-zA-Z_0-9]+)\s+:u32 = 0x;([0-9a-f]+)\s*\/\* U+([0-9A-F]{4,6}) (.*) \*\/\s*$/
 *    /^\pub const XKB_KEY_([a-zA-Z_0-9]+)\s+:u32 = 0x;([0-9a-f]+)\s*\/\*\(U+([0-9A-F]{4,6}) (.*)\)\*\/\s*$/
 *    /^\pub const XKB_KEY_([a-zA-Z_0-9]+)\s+:u32 = 0x;([0-9a-f]+)\s*(\/\*\s*(.*)\s*\*\/)?\s*$/
 *
 * Before adding new keysyms, please do consider the following: In
 * addition to the keysym names defined in this file, the
 * XStringToKeysym() and XKeysymToString() functions will also handle
 * any keysym string of the form "U0020" to "U007E" and "U00A0" to
 * "U10FFFF" for all possible Unicode characters. In other words,
 * every possible Unicode character has already a keysym string
 * defined algorithmically, even if it is not listed here. Therefore,
 * defining an additional keysym macro is only necessary where a
 * non-hexadecimal mnemonic name is needed, or where the new keysym
 * does not represent any existing Unicode character.
 *
 * When adding new keysyms to this file, do not forget to also update the
 * following as needed:
 *
 *   - the mappings in src/KeyBind.c in the libX11 repo
 *     https://gitlab.freedesktop.org/xorg/lib/libx11
 *
 *   - the protocol specification in specs/keysyms.xml in this repo
 *     https://gitlab.freedesktop.org/xorg/proto/xorgproto
 *
 */

pub const XKB_KEY_VoidSymbol                  :u32 = 0xffffff;  /* Void symbol */

/*
 * TTY function keys, cleverly chosen to map to ASCII, for convenience of
 * programming, but could have been arbitrary (at the cost of lookup
 * tables in client code).
 */

pub const XKB_KEY_BackSpace                     :u32 = 0xff08;  /* Back space, back char */
pub const XKB_KEY_Tab                           :u32 = 0xff09;
pub const XKB_KEY_Linefeed                      :u32 = 0xff0a;  /* Linefeed, LF */
pub const XKB_KEY_Clear                         :u32 = 0xff0b;
pub const XKB_KEY_Return                        :u32 = 0xff0d;  /* Return, enter */
pub const XKB_KEY_Pause                         :u32 = 0xff13;  /* Pause, hold */
pub const XKB_KEY_Scroll_Lock                   :u32 = 0xff14;
pub const XKB_KEY_Sys_Req                       :u32 = 0xff15;
pub const XKB_KEY_Escape                        :u32 = 0xff1b;
pub const XKB_KEY_Delete                        :u32 = 0xffff;  /* Delete, rubout */



/* International & multi-key character composition */

pub const XKB_KEY_Multi_key                     :u32 = 0xff20;  /* Multi-key character compose */
pub const XKB_KEY_Codeinput                     :u32 = 0xff37;
pub const XKB_KEY_SingleCandidate               :u32 = 0xff3c;
pub const XKB_KEY_MultipleCandidate             :u32 = 0xff3d;
pub const XKB_KEY_PreviousCandidate             :u32 = 0xff3e;

/* Japanese keyboard support */

pub const XKB_KEY_Kanji                         :u32 = 0xff21;  /* Kanji, Kanji convert */
pub const XKB_KEY_Muhenkan                      :u32 = 0xff22;  /* Cancel Conversion */
pub const XKB_KEY_Henkan_Mode                   :u32 = 0xff23;  /* Start/Stop Conversion */
pub const XKB_KEY_Henkan                        :u32 = 0xff23;  /* Alias for Henkan_Mode */
pub const XKB_KEY_Romaji                        :u32 = 0xff24;  /* to Romaji */
pub const XKB_KEY_Hiragana                      :u32 = 0xff25;  /* to Hiragana */
pub const XKB_KEY_Katakana                      :u32 = 0xff26;  /* to Katakana */
pub const XKB_KEY_Hiragana_Katakana             :u32 = 0xff27;  /* Hiragana/Katakana toggle */
pub const XKB_KEY_Zenkaku                       :u32 = 0xff28;  /* to Zenkaku */
pub const XKB_KEY_Hankaku                       :u32 = 0xff29;  /* to Hankaku */
pub const XKB_KEY_Zenkaku_Hankaku               :u32 = 0xff2a;  /* Zenkaku/Hankaku toggle */
pub const XKB_KEY_Touroku                       :u32 = 0xff2b;  /* Add to Dictionary */
pub const XKB_KEY_Massyo                        :u32 = 0xff2c;  /* Delete from Dictionary */
pub const XKB_KEY_Kana_Lock                     :u32 = 0xff2d;  /* Kana Lock */
pub const XKB_KEY_Kana_Shift                    :u32 = 0xff2e;  /* Kana Shift */
pub const XKB_KEY_Eisu_Shift                    :u32 = 0xff2f;  /* Alphanumeric Shift */
pub const XKB_KEY_Eisu_toggle                   :u32 = 0xff30;  /* Alphanumeric toggle */
pub const XKB_KEY_Kanji_Bangou                  :u32 = 0xff37;  /* Codeinput */
pub const XKB_KEY_Zen_Koho                      :u32 = 0xff3d;  /* Multiple/All Candidate(s) */
pub const XKB_KEY_Mae_Koho                      :u32 = 0xff3e;  /* Previous Candidate */

/* :u32 = 0xff31; thru :u32 = 0xff3f; are under XK_KOREAN */

/* Cursor control & motion */

pub const XKB_KEY_Home                          :u32 = 0xff50;
pub const XKB_KEY_Left                          :u32 = 0xff51;  /* Move left, left arrow */
pub const XKB_KEY_Up                            :u32 = 0xff52;  /* Move up, up arrow */
pub const XKB_KEY_Right                         :u32 = 0xff53;  /* Move right, right arrow */
pub const XKB_KEY_Down                          :u32 = 0xff54;  /* Move down, down arrow */
pub const XKB_KEY_Prior                         :u32 = 0xff55;  /* Prior, previous */
pub const XKB_KEY_Page_Up                       :u32 = 0xff55;
pub const XKB_KEY_Next                          :u32 = 0xff56;  /* Next */
pub const XKB_KEY_Page_Down                     :u32 = 0xff56;
pub const XKB_KEY_End                           :u32 = 0xff57;  /* EOL */
pub const XKB_KEY_Begin                         :u32 = 0xff58;  /* BOL */


/* Misc functions */

pub const XKB_KEY_Select                        :u32 = 0xff60;  /* Select, mark */
pub const XKB_KEY_Print                         :u32 = 0xff61;
pub const XKB_KEY_Execute                       :u32 = 0xff62;  /* Execute, run, do */
pub const XKB_KEY_Insert                        :u32 = 0xff63;  /* Insert, insert here */
pub const XKB_KEY_Undo                          :u32 = 0xff65;
pub const XKB_KEY_Redo                          :u32 = 0xff66;  /* Redo, again */
pub const XKB_KEY_Menu                          :u32 = 0xff67;
pub const XKB_KEY_Find                          :u32 = 0xff68;  /* Find, search */
pub const XKB_KEY_Cancel                        :u32 = 0xff69;  /* Cancel, stop, abort, exit */
pub const XKB_KEY_Help                          :u32 = 0xff6a;  /* Help */
pub const XKB_KEY_Break                         :u32 = 0xff6b;
pub const XKB_KEY_Mode_switch                   :u32 = 0xff7e;  /* Character set switch */
pub const XKB_KEY_script_switch                 :u32 = 0xff7e;  /* Alias for mode_switch */
pub const XKB_KEY_Num_Lock                      :u32 = 0xff7f;

/* Keypad functions, keypad numbers cleverly chosen to map to ASCII */

pub const XKB_KEY_KP_Space                      :u32 = 0xff80;  /* Space */
pub const XKB_KEY_KP_Tab                        :u32 = 0xff89;
pub const XKB_KEY_KP_Enter                      :u32 = 0xff8d;  /* Enter */
pub const XKB_KEY_KP_F1                         :u32 = 0xff91;  /* PF1, KP_A, ... */
pub const XKB_KEY_KP_F2                         :u32 = 0xff92;
pub const XKB_KEY_KP_F3                         :u32 = 0xff93;
pub const XKB_KEY_KP_F4                         :u32 = 0xff94;
pub const XKB_KEY_KP_Home                       :u32 = 0xff95;
pub const XKB_KEY_KP_Left                       :u32 = 0xff96;
pub const XKB_KEY_KP_Up                         :u32 = 0xff97;
pub const XKB_KEY_KP_Right                      :u32 = 0xff98;
pub const XKB_KEY_KP_Down                       :u32 = 0xff99;
pub const XKB_KEY_KP_Prior                      :u32 = 0xff9a;
pub const XKB_KEY_KP_Page_Up                    :u32 = 0xff9a;
pub const XKB_KEY_KP_Next                       :u32 = 0xff9b;
pub const XKB_KEY_KP_Page_Down                  :u32 = 0xff9b;
pub const XKB_KEY_KP_End                        :u32 = 0xff9c;
pub const XKB_KEY_KP_Begin                      :u32 = 0xff9d;
pub const XKB_KEY_KP_Insert                     :u32 = 0xff9e;
pub const XKB_KEY_KP_Delete                     :u32 = 0xff9f;
pub const XKB_KEY_KP_Equal                      :u32 = 0xffbd;  /* Equals */
pub const XKB_KEY_KP_Multiply                   :u32 = 0xffaa;
pub const XKB_KEY_KP_Add                        :u32 = 0xffab;
pub const XKB_KEY_KP_Separator                  :u32 = 0xffac;  /* Separator, often comma */
pub const XKB_KEY_KP_Subtract                   :u32 = 0xffad;
pub const XKB_KEY_KP_Decimal                    :u32 = 0xffae;
pub const XKB_KEY_KP_Divide                     :u32 = 0xffaf;

pub const XKB_KEY_KP_0                          :u32 = 0xffb0;
pub const XKB_KEY_KP_1                          :u32 = 0xffb1;
pub const XKB_KEY_KP_2                          :u32 = 0xffb2;
pub const XKB_KEY_KP_3                          :u32 = 0xffb3;
pub const XKB_KEY_KP_4                          :u32 = 0xffb4;
pub const XKB_KEY_KP_5                          :u32 = 0xffb5;
pub const XKB_KEY_KP_6                          :u32 = 0xffb6;
pub const XKB_KEY_KP_7                          :u32 = 0xffb7;
pub const XKB_KEY_KP_8                          :u32 = 0xffb8;
pub const XKB_KEY_KP_9                          :u32 = 0xffb9;



/*
 * Auxiliary functions; note the duplicate definitions for left and right
 * function keys;  Sun keyboards and a few other manufacturers have such
 * function key groups on the left and/or right sides of the keyboard.
 * We've not found a keyboard with more than 35 function keys total.
 */

pub const XKB_KEY_F1                            :u32 = 0xffbe;
pub const XKB_KEY_F2                            :u32 = 0xffbf;
pub const XKB_KEY_F3                            :u32 = 0xffc0;
pub const XKB_KEY_F4                            :u32 = 0xffc1;
pub const XKB_KEY_F5                            :u32 = 0xffc2;
pub const XKB_KEY_F6                            :u32 = 0xffc3;
pub const XKB_KEY_F7                            :u32 = 0xffc4;
pub const XKB_KEY_F8                            :u32 = 0xffc5;
pub const XKB_KEY_F9                            :u32 = 0xffc6;
pub const XKB_KEY_F10                           :u32 = 0xffc7;
pub const XKB_KEY_F11                           :u32 = 0xffc8;
pub const XKB_KEY_L1                            :u32 = 0xffc8;
pub const XKB_KEY_F12                           :u32 = 0xffc9;
pub const XKB_KEY_L2                            :u32 = 0xffc9;
pub const XKB_KEY_F13                           :u32 = 0xffca;
pub const XKB_KEY_L3                            :u32 = 0xffca;
pub const XKB_KEY_F14                           :u32 = 0xffcb;
pub const XKB_KEY_L4                            :u32 = 0xffcb;
pub const XKB_KEY_F15                           :u32 = 0xffcc;
pub const XKB_KEY_L5                            :u32 = 0xffcc;
pub const XKB_KEY_F16                           :u32 = 0xffcd;
pub const XKB_KEY_L6                            :u32 = 0xffcd;
pub const XKB_KEY_F17                           :u32 = 0xffce;
pub const XKB_KEY_L7                            :u32 = 0xffce;
pub const XKB_KEY_F18                           :u32 = 0xffcf;
pub const XKB_KEY_L8                            :u32 = 0xffcf;
pub const XKB_KEY_F19                           :u32 = 0xffd0;
pub const XKB_KEY_L9                            :u32 = 0xffd0;
pub const XKB_KEY_F20                           :u32 = 0xffd1;
pub const XKB_KEY_L10                           :u32 = 0xffd1;
pub const XKB_KEY_F21                           :u32 = 0xffd2;
pub const XKB_KEY_R1                            :u32 = 0xffd2;
pub const XKB_KEY_F22                           :u32 = 0xffd3;
pub const XKB_KEY_R2                            :u32 = 0xffd3;
pub const XKB_KEY_F23                           :u32 = 0xffd4;
pub const XKB_KEY_R3                            :u32 = 0xffd4;
pub const XKB_KEY_F24                           :u32 = 0xffd5;
pub const XKB_KEY_R4                            :u32 = 0xffd5;
pub const XKB_KEY_F25                           :u32 = 0xffd6;
pub const XKB_KEY_R5                            :u32 = 0xffd6;
pub const XKB_KEY_F26                           :u32 = 0xffd7;
pub const XKB_KEY_R6                            :u32 = 0xffd7;
pub const XKB_KEY_F27                           :u32 = 0xffd8;
pub const XKB_KEY_R7                            :u32 = 0xffd8;
pub const XKB_KEY_F28                           :u32 = 0xffd9;
pub const XKB_KEY_R8                            :u32 = 0xffd9;
pub const XKB_KEY_F29                           :u32 = 0xffda;
pub const XKB_KEY_R9                            :u32 = 0xffda;
pub const XKB_KEY_F30                           :u32 = 0xffdb;
pub const XKB_KEY_R10                           :u32 = 0xffdb;
pub const XKB_KEY_F31                           :u32 = 0xffdc;
pub const XKB_KEY_R11                           :u32 = 0xffdc;
pub const XKB_KEY_F32                           :u32 = 0xffdd;
pub const XKB_KEY_R12                           :u32 = 0xffdd;
pub const XKB_KEY_F33                           :u32 = 0xffde;
pub const XKB_KEY_R13                           :u32 = 0xffde;
pub const XKB_KEY_F34                           :u32 = 0xffdf;
pub const XKB_KEY_R14                           :u32 = 0xffdf;
pub const XKB_KEY_F35                           :u32 = 0xffe0;
pub const XKB_KEY_R15                           :u32 = 0xffe0;

/* Modifiers */

pub const XKB_KEY_Shift_L                       :u32 = 0xffe1;  /* Left shift */
pub const XKB_KEY_Shift_R                       :u32 = 0xffe2;  /* Right shift */
pub const XKB_KEY_Control_L                     :u32 = 0xffe3;  /* Left control */
pub const XKB_KEY_Control_R                     :u32 = 0xffe4;  /* Right control */
pub const XKB_KEY_Caps_Lock                     :u32 = 0xffe5;  /* Caps lock */
pub const XKB_KEY_Shift_Lock                    :u32 = 0xffe6;  /* Shift lock */

pub const XKB_KEY_Meta_L                        :u32 = 0xffe7;  /* Left meta */
pub const XKB_KEY_Meta_R                        :u32 = 0xffe8;  /* Right meta */
pub const XKB_KEY_Alt_L                         :u32 = 0xffe9;  /* Left alt */
pub const XKB_KEY_Alt_R                         :u32 = 0xffea;  /* Right alt */
pub const XKB_KEY_Super_L                       :u32 = 0xffeb;  /* Left super */
pub const XKB_KEY_Super_R                       :u32 = 0xffec;  /* Right super */
pub const XKB_KEY_Hyper_L                       :u32 = 0xffed;  /* Left hyper */
pub const XKB_KEY_Hyper_R                       :u32 = 0xffee;  /* Right hyper */

/*
 * Keyboard (XKB) Extension function and modifier keys
 * (from Appendix C of "The X Keyboard Extension: Protocol Specification")
 * Byte 3 = :u32 = 0xfe;
 */

pub const XKB_KEY_ISO_Lock                      :u32 = 0xfe01;
pub const XKB_KEY_ISO_Level2_Latch              :u32 = 0xfe02;
pub const XKB_KEY_ISO_Level3_Shift              :u32 = 0xfe03;
pub const XKB_KEY_ISO_Level3_Latch              :u32 = 0xfe04;
pub const XKB_KEY_ISO_Level3_Lock               :u32 = 0xfe05;
pub const XKB_KEY_ISO_Level5_Shift              :u32 = 0xfe11;
pub const XKB_KEY_ISO_Level5_Latch              :u32 = 0xfe12;
pub const XKB_KEY_ISO_Level5_Lock               :u32 = 0xfe13;
pub const XKB_KEY_ISO_Group_Shift               :u32 = 0xff7e;  /* Alias for mode_switch */
pub const XKB_KEY_ISO_Group_Latch               :u32 = 0xfe06;
pub const XKB_KEY_ISO_Group_Lock                :u32 = 0xfe07;
pub const XKB_KEY_ISO_Next_Group                :u32 = 0xfe08;
pub const XKB_KEY_ISO_Next_Group_Lock           :u32 = 0xfe09;
pub const XKB_KEY_ISO_Prev_Group                :u32 = 0xfe0a;
pub const XKB_KEY_ISO_Prev_Group_Lock           :u32 = 0xfe0b;
pub const XKB_KEY_ISO_First_Group               :u32 = 0xfe0c;
pub const XKB_KEY_ISO_First_Group_Lock          :u32 = 0xfe0d;
pub const XKB_KEY_ISO_Last_Group                :u32 = 0xfe0e;
pub const XKB_KEY_ISO_Last_Group_Lock           :u32 = 0xfe0f;

pub const XKB_KEY_ISO_Left_Tab                  :u32 = 0xfe20;
pub const XKB_KEY_ISO_Move_Line_Up              :u32 = 0xfe21;
pub const XKB_KEY_ISO_Move_Line_Down            :u32 = 0xfe22;
pub const XKB_KEY_ISO_Partial_Line_Up           :u32 = 0xfe23;
pub const XKB_KEY_ISO_Partial_Line_Down         :u32 = 0xfe24;
pub const XKB_KEY_ISO_Partial_Space_Left        :u32 = 0xfe25;
pub const XKB_KEY_ISO_Partial_Space_Right       :u32 = 0xfe26;
pub const XKB_KEY_ISO_Set_Margin_Left           :u32 = 0xfe27;
pub const XKB_KEY_ISO_Set_Margin_Right          :u32 = 0xfe28;
pub const XKB_KEY_ISO_Release_Margin_Left       :u32 = 0xfe29;
pub const XKB_KEY_ISO_Release_Margin_Right      :u32 = 0xfe2a;
pub const XKB_KEY_ISO_Release_Both_Margins      :u32 = 0xfe2b;
pub const XKB_KEY_ISO_Fast_Cursor_Left          :u32 = 0xfe2c;
pub const XKB_KEY_ISO_Fast_Cursor_Right         :u32 = 0xfe2d;
pub const XKB_KEY_ISO_Fast_Cursor_Up            :u32 = 0xfe2e;
pub const XKB_KEY_ISO_Fast_Cursor_Down          :u32 = 0xfe2f;
pub const XKB_KEY_ISO_Continuous_Underline      :u32 = 0xfe30;
pub const XKB_KEY_ISO_Discontinuous_Underline   :u32 = 0xfe31;
pub const XKB_KEY_ISO_Emphasize                 :u32 = 0xfe32;
pub const XKB_KEY_ISO_Center_Object             :u32 = 0xfe33;
pub const XKB_KEY_ISO_Enter                     :u32 = 0xfe34;

pub const XKB_KEY_dead_grave                    :u32 = 0xfe50;
pub const XKB_KEY_dead_acute                    :u32 = 0xfe51;
pub const XKB_KEY_dead_circumflex               :u32 = 0xfe52;
pub const XKB_KEY_dead_tilde                    :u32 = 0xfe53;
pub const XKB_KEY_dead_perispomeni              :u32 = 0xfe53;  /* alias for dead_tilde */
pub const XKB_KEY_dead_macron                   :u32 = 0xfe54;
pub const XKB_KEY_dead_breve                    :u32 = 0xfe55;
pub const XKB_KEY_dead_abovedot                 :u32 = 0xfe56;
pub const XKB_KEY_dead_diaeresis                :u32 = 0xfe57;
pub const XKB_KEY_dead_abovering                :u32 = 0xfe58;
pub const XKB_KEY_dead_doubleacute              :u32 = 0xfe59;
pub const XKB_KEY_dead_caron                    :u32 = 0xfe5a;
pub const XKB_KEY_dead_cedilla                  :u32 = 0xfe5b;
pub const XKB_KEY_dead_ogonek                   :u32 = 0xfe5c;
pub const XKB_KEY_dead_iota                     :u32 = 0xfe5d;
pub const XKB_KEY_dead_voiced_sound             :u32 = 0xfe5e;
pub const XKB_KEY_dead_semivoiced_sound         :u32 = 0xfe5f;
pub const XKB_KEY_dead_belowdot                 :u32 = 0xfe60;
pub const XKB_KEY_dead_hook                     :u32 = 0xfe61;
pub const XKB_KEY_dead_horn                     :u32 = 0xfe62;
pub const XKB_KEY_dead_stroke                   :u32 = 0xfe63;
pub const XKB_KEY_dead_abovecomma               :u32 = 0xfe64;
pub const XKB_KEY_dead_psili                    :u32 = 0xfe64;  /* alias for dead_abovecomma */
pub const XKB_KEY_dead_abovereversedcomma       :u32 = 0xfe65;
pub const XKB_KEY_dead_dasia                    :u32 = 0xfe65;  /* alias for dead_abovereversedcomma */
pub const XKB_KEY_dead_doublegrave              :u32 = 0xfe66;
pub const XKB_KEY_dead_belowring                :u32 = 0xfe67;
pub const XKB_KEY_dead_belowmacron              :u32 = 0xfe68;
pub const XKB_KEY_dead_belowcircumflex          :u32 = 0xfe69;
pub const XKB_KEY_dead_belowtilde               :u32 = 0xfe6a;
pub const XKB_KEY_dead_belowbreve               :u32 = 0xfe6b;
pub const XKB_KEY_dead_belowdiaeresis           :u32 = 0xfe6c;
pub const XKB_KEY_dead_invertedbreve            :u32 = 0xfe6d;
pub const XKB_KEY_dead_belowcomma               :u32 = 0xfe6e;
pub const XKB_KEY_dead_currency                 :u32 = 0xfe6f;

/* extra dead elements for German T3 layout */
pub const XKB_KEY_dead_lowline                  :u32 = 0xfe90;
pub const XKB_KEY_dead_aboveverticalline        :u32 = 0xfe91;
pub const XKB_KEY_dead_belowverticalline        :u32 = 0xfe92;
pub const XKB_KEY_dead_longsolidusoverlay       :u32 = 0xfe93;

/* dead vowels for universal syllable entry */
pub const XKB_KEY_dead_a                        :u32 = 0xfe80;
pub const XKB_KEY_dead_A                        :u32 = 0xfe81;
pub const XKB_KEY_dead_e                        :u32 = 0xfe82;
pub const XKB_KEY_dead_E                        :u32 = 0xfe83;
pub const XKB_KEY_dead_i                        :u32 = 0xfe84;
pub const XKB_KEY_dead_I                        :u32 = 0xfe85;
pub const XKB_KEY_dead_o                        :u32 = 0xfe86;
pub const XKB_KEY_dead_O                        :u32 = 0xfe87;
pub const XKB_KEY_dead_u                        :u32 = 0xfe88;
pub const XKB_KEY_dead_U                        :u32 = 0xfe89;
pub const XKB_KEY_dead_small_schwa              :u32 = 0xfe8a;
pub const XKB_KEY_dead_capital_schwa            :u32 = 0xfe8b;

pub const XKB_KEY_dead_greek                    :u32 = 0xfe8c;

pub const XKB_KEY_First_Virtual_Screen          :u32 = 0xfed0;
pub const XKB_KEY_Prev_Virtual_Screen           :u32 = 0xfed1;
pub const XKB_KEY_Next_Virtual_Screen           :u32 = 0xfed2;
pub const XKB_KEY_Last_Virtual_Screen           :u32 = 0xfed4;
pub const XKB_KEY_Terminate_Server              :u32 = 0xfed5;

pub const XKB_KEY_AccessX_Enable                :u32 = 0xfe70;
pub const XKB_KEY_AccessX_Feedback_Enable       :u32 = 0xfe71;
pub const XKB_KEY_RepeatKeys_Enable             :u32 = 0xfe72;
pub const XKB_KEY_SlowKeys_Enable               :u32 = 0xfe73;
pub const XKB_KEY_BounceKeys_Enable             :u32 = 0xfe74;
pub const XKB_KEY_StickyKeys_Enable             :u32 = 0xfe75;
pub const XKB_KEY_MouseKeys_Enable              :u32 = 0xfe76;
pub const XKB_KEY_MouseKeys_Accel_Enable        :u32 = 0xfe77;
pub const XKB_KEY_Overlay1_Enable               :u32 = 0xfe78;
pub const XKB_KEY_Overlay2_Enable               :u32 = 0xfe79;
pub const XKB_KEY_AudibleBell_Enable            :u32 = 0xfe7a;

pub const XKB_KEY_Pointer_Left                  :u32 = 0xfee0;
pub const XKB_KEY_Pointer_Right                 :u32 = 0xfee1;
pub const XKB_KEY_Pointer_Up                    :u32 = 0xfee2;
pub const XKB_KEY_Pointer_Down                  :u32 = 0xfee3;
pub const XKB_KEY_Pointer_UpLeft                :u32 = 0xfee4;
pub const XKB_KEY_Pointer_UpRight               :u32 = 0xfee5;
pub const XKB_KEY_Pointer_DownLeft              :u32 = 0xfee6;
pub const XKB_KEY_Pointer_DownRight             :u32 = 0xfee7;
pub const XKB_KEY_Pointer_Button_Dflt           :u32 = 0xfee8;
pub const XKB_KEY_Pointer_Button1               :u32 = 0xfee9;
pub const XKB_KEY_Pointer_Button2               :u32 = 0xfeea;
pub const XKB_KEY_Pointer_Button3               :u32 = 0xfeeb;
pub const XKB_KEY_Pointer_Button4               :u32 = 0xfeec;
pub const XKB_KEY_Pointer_Button5               :u32 = 0xfeed;
pub const XKB_KEY_Pointer_DblClick_Dflt         :u32 = 0xfeee;
pub const XKB_KEY_Pointer_DblClick1             :u32 = 0xfeef;
pub const XKB_KEY_Pointer_DblClick2             :u32 = 0xfef0;
pub const XKB_KEY_Pointer_DblClick3             :u32 = 0xfef1;
pub const XKB_KEY_Pointer_DblClick4             :u32 = 0xfef2;
pub const XKB_KEY_Pointer_DblClick5             :u32 = 0xfef3;
pub const XKB_KEY_Pointer_Drag_Dflt             :u32 = 0xfef4;
pub const XKB_KEY_Pointer_Drag1                 :u32 = 0xfef5;
pub const XKB_KEY_Pointer_Drag2                 :u32 = 0xfef6;
pub const XKB_KEY_Pointer_Drag3                 :u32 = 0xfef7;
pub const XKB_KEY_Pointer_Drag4                 :u32 = 0xfef8;
pub const XKB_KEY_Pointer_Drag5                 :u32 = 0xfefd;

pub const XKB_KEY_Pointer_EnableKeys            :u32 = 0xfef9;
pub const XKB_KEY_Pointer_Accelerate            :u32 = 0xfefa;
pub const XKB_KEY_Pointer_DfltBtnNext           :u32 = 0xfefb;
pub const XKB_KEY_Pointer_DfltBtnPrev           :u32 = 0xfefc;

/* Single-Stroke Multiple-Character N-Graph Keysyms For The X Input Method */

pub const XKB_KEY_ch                            :u32 = 0xfea0;
pub const XKB_KEY_Ch                            :u32 = 0xfea1;
pub const XKB_KEY_CH                            :u32 = 0xfea2;
pub const XKB_KEY_c_h                           :u32 = 0xfea3;
pub const XKB_KEY_C_h                           :u32 = 0xfea4;
pub const XKB_KEY_C_H                           :u32 = 0xfea5;


/*
 * 3270 Terminal Keys
 * Byte 3 = :u32 = 0xfd;
 */

pub const XKB_KEY_3270_Duplicate                :u32 = 0xfd01;
pub const XKB_KEY_3270_FieldMark                :u32 = 0xfd02;
pub const XKB_KEY_3270_Right2                   :u32 = 0xfd03;
pub const XKB_KEY_3270_Left2                    :u32 = 0xfd04;
pub const XKB_KEY_3270_BackTab                  :u32 = 0xfd05;
pub const XKB_KEY_3270_EraseEOF                 :u32 = 0xfd06;
pub const XKB_KEY_3270_EraseInput               :u32 = 0xfd07;
pub const XKB_KEY_3270_Reset                    :u32 = 0xfd08;
pub const XKB_KEY_3270_Quit                     :u32 = 0xfd09;
pub const XKB_KEY_3270_PA1                      :u32 = 0xfd0a;
pub const XKB_KEY_3270_PA2                      :u32 = 0xfd0b;
pub const XKB_KEY_3270_PA3                      :u32 = 0xfd0c;
pub const XKB_KEY_3270_Test                     :u32 = 0xfd0d;
pub const XKB_KEY_3270_Attn                     :u32 = 0xfd0e;
pub const XKB_KEY_3270_CursorBlink              :u32 = 0xfd0f;
pub const XKB_KEY_3270_AltCursor                :u32 = 0xfd10;
pub const XKB_KEY_3270_KeyClick                 :u32 = 0xfd11;
pub const XKB_KEY_3270_Jump                     :u32 = 0xfd12;
pub const XKB_KEY_3270_Ident                    :u32 = 0xfd13;
pub const XKB_KEY_3270_Rule                     :u32 = 0xfd14;
pub const XKB_KEY_3270_Copy                     :u32 = 0xfd15;
pub const XKB_KEY_3270_Play                     :u32 = 0xfd16;
pub const XKB_KEY_3270_Setup                    :u32 = 0xfd17;
pub const XKB_KEY_3270_Record                   :u32 = 0xfd18;
pub const XKB_KEY_3270_ChangeScreen             :u32 = 0xfd19;
pub const XKB_KEY_3270_DeleteWord               :u32 = 0xfd1a;
pub const XKB_KEY_3270_ExSelect                 :u32 = 0xfd1b;
pub const XKB_KEY_3270_CursorSelect             :u32 = 0xfd1c;
pub const XKB_KEY_3270_PrintScreen              :u32 = 0xfd1d;
pub const XKB_KEY_3270_Enter                    :u32 = 0xfd1e;

/*
 * Latin 1
 * (ISO/IEC 8859-1 = Unicode U+0020..U+00FF)
 * Byte 3 = 0
 */
pub const XKB_KEY_space                         :u32 = 0x0020;  /* U+0020 SPACE */
pub const XKB_KEY_exclam                        :u32 = 0x0021;  /* U+0021 EXCLAMATION MARK */
pub const XKB_KEY_quotedbl                      :u32 = 0x0022;  /* U+0022 QUOTATION MARK */
pub const XKB_KEY_numbersign                    :u32 = 0x0023;  /* U+0023 NUMBER SIGN */
pub const XKB_KEY_dollar                        :u32 = 0x0024;  /* U+0024 DOLLAR SIGN */
pub const XKB_KEY_percent                       :u32 = 0x0025;  /* U+0025 PERCENT SIGN */
pub const XKB_KEY_ampersand                     :u32 = 0x0026;  /* U+0026 AMPERSAND */
pub const XKB_KEY_apostrophe                    :u32 = 0x0027;  /* U+0027 APOSTROPHE */
pub const XKB_KEY_quoteright                    :u32 = 0x0027;  /* deprecated */
pub const XKB_KEY_parenleft                     :u32 = 0x0028;  /* U+0028 LEFT PARENTHESIS */
pub const XKB_KEY_parenright                    :u32 = 0x0029;  /* U+0029 RIGHT PARENTHESIS */
pub const XKB_KEY_asterisk                      :u32 = 0x002a;  /* U+002A ASTERISK */
pub const XKB_KEY_plus                          :u32 = 0x002b;  /* U+002B PLUS SIGN */
pub const XKB_KEY_comma                         :u32 = 0x002c;  /* U+002C COMMA */
pub const XKB_KEY_minus                         :u32 = 0x002d;  /* U+002D HYPHEN-MINUS */
pub const XKB_KEY_period                        :u32 = 0x002e;  /* U+002E FULL STOP */
pub const XKB_KEY_slash                         :u32 = 0x002f;  /* U+002F SOLIDUS */
pub const XKB_KEY_0                             :u32 = 0x0030;  /* U+0030 DIGIT ZERO */
pub const XKB_KEY_1                             :u32 = 0x0031;  /* U+0031 DIGIT ONE */
pub const XKB_KEY_2                             :u32 = 0x0032;  /* U+0032 DIGIT TWO */
pub const XKB_KEY_3                             :u32 = 0x0033;  /* U+0033 DIGIT THREE */
pub const XKB_KEY_4                             :u32 = 0x0034;  /* U+0034 DIGIT FOUR */
pub const XKB_KEY_5                             :u32 = 0x0035;  /* U+0035 DIGIT FIVE */
pub const XKB_KEY_6                             :u32 = 0x0036;  /* U+0036 DIGIT SIX */
pub const XKB_KEY_7                             :u32 = 0x0037;  /* U+0037 DIGIT SEVEN */
pub const XKB_KEY_8                             :u32 = 0x0038;  /* U+0038 DIGIT EIGHT */
pub const XKB_KEY_9                             :u32 = 0x0039;  /* U+0039 DIGIT NINE */
pub const XKB_KEY_colon                         :u32 = 0x003a;  /* U+003A COLON */
pub const XKB_KEY_semicolon                     :u32 = 0x003b;  /* U+003B SEMICOLON */
pub const XKB_KEY_less                          :u32 = 0x003c;  /* U+003C LESS-THAN SIGN */
pub const XKB_KEY_equal                         :u32 = 0x003d;  /* U+003D EQUALS SIGN */
pub const XKB_KEY_greater                       :u32 = 0x003e;  /* U+003E GREATER-THAN SIGN */
pub const XKB_KEY_question                      :u32 = 0x003f;  /* U+003F QUESTION MARK */
pub const XKB_KEY_at                            :u32 = 0x0040;  /* U+0040 COMMERCIAL AT */
pub const XKB_KEY_A                             :u32 = 0x0041;  /* U+0041 LATIN CAPITAL LETTER A */
pub const XKB_KEY_B                             :u32 = 0x0042;  /* U+0042 LATIN CAPITAL LETTER B */
pub const XKB_KEY_C                             :u32 = 0x0043;  /* U+0043 LATIN CAPITAL LETTER C */
pub const XKB_KEY_D                             :u32 = 0x0044;  /* U+0044 LATIN CAPITAL LETTER D */
pub const XKB_KEY_E                             :u32 = 0x0045;  /* U+0045 LATIN CAPITAL LETTER E */
pub const XKB_KEY_F                             :u32 = 0x0046;  /* U+0046 LATIN CAPITAL LETTER F */
pub const XKB_KEY_G                             :u32 = 0x0047;  /* U+0047 LATIN CAPITAL LETTER G */
pub const XKB_KEY_H                             :u32 = 0x0048;  /* U+0048 LATIN CAPITAL LETTER H */
pub const XKB_KEY_I                             :u32 = 0x0049;  /* U+0049 LATIN CAPITAL LETTER I */
pub const XKB_KEY_J                             :u32 = 0x004a;  /* U+004A LATIN CAPITAL LETTER J */
pub const XKB_KEY_K                             :u32 = 0x004b;  /* U+004B LATIN CAPITAL LETTER K */
pub const XKB_KEY_L                             :u32 = 0x004c;  /* U+004C LATIN CAPITAL LETTER L */
pub const XKB_KEY_M                             :u32 = 0x004d;  /* U+004D LATIN CAPITAL LETTER M */
pub const XKB_KEY_N                             :u32 = 0x004e;  /* U+004E LATIN CAPITAL LETTER N */
pub const XKB_KEY_O                             :u32 = 0x004f;  /* U+004F LATIN CAPITAL LETTER O */
pub const XKB_KEY_P                             :u32 = 0x0050;  /* U+0050 LATIN CAPITAL LETTER P */
pub const XKB_KEY_Q                             :u32 = 0x0051;  /* U+0051 LATIN CAPITAL LETTER Q */
pub const XKB_KEY_R                             :u32 = 0x0052;  /* U+0052 LATIN CAPITAL LETTER R */
pub const XKB_KEY_S                             :u32 = 0x0053;  /* U+0053 LATIN CAPITAL LETTER S */
pub const XKB_KEY_T                             :u32 = 0x0054;  /* U+0054 LATIN CAPITAL LETTER T */
pub const XKB_KEY_U                             :u32 = 0x0055;  /* U+0055 LATIN CAPITAL LETTER U */
pub const XKB_KEY_V                             :u32 = 0x0056;  /* U+0056 LATIN CAPITAL LETTER V */
pub const XKB_KEY_W                             :u32 = 0x0057;  /* U+0057 LATIN CAPITAL LETTER W */
pub const XKB_KEY_X                             :u32 = 0x0058;  /* U+0058 LATIN CAPITAL LETTER X */
pub const XKB_KEY_Y                             :u32 = 0x0059;  /* U+0059 LATIN CAPITAL LETTER Y */
pub const XKB_KEY_Z                             :u32 = 0x005a;  /* U+005A LATIN CAPITAL LETTER Z */
pub const XKB_KEY_bracketleft                   :u32 = 0x005b;  /* U+005B LEFT SQUARE BRACKET */
pub const XKB_KEY_backslash                     :u32 = 0x005c;  /* U+005C REVERSE SOLIDUS */
pub const XKB_KEY_bracketright                  :u32 = 0x005d;  /* U+005D RIGHT SQUARE BRACKET */
pub const XKB_KEY_asciicircum                   :u32 = 0x005e;  /* U+005E CIRCUMFLEX ACCENT */
pub const XKB_KEY_underscore                    :u32 = 0x005f;  /* U+005F LOW LINE */
pub const XKB_KEY_grave                         :u32 = 0x0060;  /* U+0060 GRAVE ACCENT */
pub const XKB_KEY_quoteleft                     :u32 = 0x0060;  /* deprecated */
pub const XKB_KEY_a                             :u32 = 0x0061;  /* U+0061 LATIN SMALL LETTER A */
pub const XKB_KEY_b                             :u32 = 0x0062;  /* U+0062 LATIN SMALL LETTER B */
pub const XKB_KEY_c                             :u32 = 0x0063;  /* U+0063 LATIN SMALL LETTER C */
pub const XKB_KEY_d                             :u32 = 0x0064;  /* U+0064 LATIN SMALL LETTER D */
pub const XKB_KEY_e                             :u32 = 0x0065;  /* U+0065 LATIN SMALL LETTER E */
pub const XKB_KEY_f                             :u32 = 0x0066;  /* U+0066 LATIN SMALL LETTER F */
pub const XKB_KEY_g                             :u32 = 0x0067;  /* U+0067 LATIN SMALL LETTER G */
pub const XKB_KEY_h                             :u32 = 0x0068;  /* U+0068 LATIN SMALL LETTER H */
pub const XKB_KEY_i                             :u32 = 0x0069;  /* U+0069 LATIN SMALL LETTER I */
pub const XKB_KEY_j                             :u32 = 0x006a;  /* U+006A LATIN SMALL LETTER J */
pub const XKB_KEY_k                             :u32 = 0x006b;  /* U+006B LATIN SMALL LETTER K */
pub const XKB_KEY_l                             :u32 = 0x006c;  /* U+006C LATIN SMALL LETTER L */
pub const XKB_KEY_m                             :u32 = 0x006d;  /* U+006D LATIN SMALL LETTER M */
pub const XKB_KEY_n                             :u32 = 0x006e;  /* U+006E LATIN SMALL LETTER N */
pub const XKB_KEY_o                             :u32 = 0x006f;  /* U+006F LATIN SMALL LETTER O */
pub const XKB_KEY_p                             :u32 = 0x0070;  /* U+0070 LATIN SMALL LETTER P */
pub const XKB_KEY_q                             :u32 = 0x0071;  /* U+0071 LATIN SMALL LETTER Q */
pub const XKB_KEY_r                             :u32 = 0x0072;  /* U+0072 LATIN SMALL LETTER R */
pub const XKB_KEY_s                             :u32 = 0x0073;  /* U+0073 LATIN SMALL LETTER S */
pub const XKB_KEY_t                             :u32 = 0x0074;  /* U+0074 LATIN SMALL LETTER T */
pub const XKB_KEY_u                             :u32 = 0x0075;  /* U+0075 LATIN SMALL LETTER U */
pub const XKB_KEY_v                             :u32 = 0x0076;  /* U+0076 LATIN SMALL LETTER V */
pub const XKB_KEY_w                             :u32 = 0x0077;  /* U+0077 LATIN SMALL LETTER W */
pub const XKB_KEY_x                             :u32 = 0x0078;  /* U+0078 LATIN SMALL LETTER X */
pub const XKB_KEY_y                             :u32 = 0x0079;  /* U+0079 LATIN SMALL LETTER Y */
pub const XKB_KEY_z                             :u32 = 0x007a;  /* U+007A LATIN SMALL LETTER Z */
pub const XKB_KEY_braceleft                     :u32 = 0x007b;  /* U+007B LEFT CURLY BRACKET */
pub const XKB_KEY_bar                           :u32 = 0x007c;  /* U+007C VERTICAL LINE */
pub const XKB_KEY_braceright                    :u32 = 0x007d;  /* U+007D RIGHT CURLY BRACKET */
pub const XKB_KEY_asciitilde                    :u32 = 0x007e;  /* U+007E TILDE */

pub const XKB_KEY_nobreakspace                  :u32 = 0x00a0;  /* U+00A0 NO-BREAK SPACE */
pub const XKB_KEY_exclamdown                    :u32 = 0x00a1;  /* U+00A1 INVERTED EXCLAMATION MARK */
pub const XKB_KEY_cent                          :u32 = 0x00a2;  /* U+00A2 CENT SIGN */
pub const XKB_KEY_sterling                      :u32 = 0x00a3;  /* U+00A3 POUND SIGN */
pub const XKB_KEY_currency                      :u32 = 0x00a4;  /* U+00A4 CURRENCY SIGN */
pub const XKB_KEY_yen                           :u32 = 0x00a5;  /* U+00A5 YEN SIGN */
pub const XKB_KEY_brokenbar                     :u32 = 0x00a6;  /* U+00A6 BROKEN BAR */
pub const XKB_KEY_section                       :u32 = 0x00a7;  /* U+00A7 SECTION SIGN */
pub const XKB_KEY_diaeresis                     :u32 = 0x00a8;  /* U+00A8 DIAERESIS */
pub const XKB_KEY_copyright                     :u32 = 0x00a9;  /* U+00A9 COPYRIGHT SIGN */
pub const XKB_KEY_ordfeminine                   :u32 = 0x00aa;  /* U+00AA FEMININE ORDINAL INDICATOR */
pub const XKB_KEY_guillemotleft                 :u32 = 0x00ab;  /* U+00AB LEFT-POINTING DOUBLE ANGLE QUOTATION MARK */
pub const XKB_KEY_notsign                       :u32 = 0x00ac;  /* U+00AC NOT SIGN */
pub const XKB_KEY_hyphen                        :u32 = 0x00ad;  /* U+00AD SOFT HYPHEN */
pub const XKB_KEY_registered                    :u32 = 0x00ae;  /* U+00AE REGISTERED SIGN */
pub const XKB_KEY_macron                        :u32 = 0x00af;  /* U+00AF MACRON */
pub const XKB_KEY_degree                        :u32 = 0x00b0;  /* U+00B0 DEGREE SIGN */
pub const XKB_KEY_plusminus                     :u32 = 0x00b1;  /* U+00B1 PLUS-MINUS SIGN */
pub const XKB_KEY_twosuperior                   :u32 = 0x00b2;  /* U+00B2 SUPERSCRIPT TWO */
pub const XKB_KEY_threesuperior                 :u32 = 0x00b3;  /* U+00B3 SUPERSCRIPT THREE */
pub const XKB_KEY_acute                         :u32 = 0x00b4;  /* U+00B4 ACUTE ACCENT */
pub const XKB_KEY_mu                            :u32 = 0x00b5;  /* U+00B5 MICRO SIGN */
pub const XKB_KEY_paragraph                     :u32 = 0x00b6;  /* U+00B6 PILCROW SIGN */
pub const XKB_KEY_periodcentered                :u32 = 0x00b7;  /* U+00B7 MIDDLE DOT */
pub const XKB_KEY_cedilla                       :u32 = 0x00b8;  /* U+00B8 CEDILLA */
pub const XKB_KEY_onesuperior                   :u32 = 0x00b9;  /* U+00B9 SUPERSCRIPT ONE */
pub const XKB_KEY_masculine                     :u32 = 0x00ba;  /* U+00BA MASCULINE ORDINAL INDICATOR */
pub const XKB_KEY_guillemotright                :u32 = 0x00bb;  /* U+00BB RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK */
pub const XKB_KEY_onequarter                    :u32 = 0x00bc;  /* U+00BC VULGAR FRACTION ONE QUARTER */
pub const XKB_KEY_onehalf                       :u32 = 0x00bd;  /* U+00BD VULGAR FRACTION ONE HALF */
pub const XKB_KEY_threequarters                 :u32 = 0x00be;  /* U+00BE VULGAR FRACTION THREE QUARTERS */
pub const XKB_KEY_questiondown                  :u32 = 0x00bf;  /* U+00BF INVERTED QUESTION MARK */
pub const XKB_KEY_Agrave                        :u32 = 0x00c0;  /* U+00C0 LATIN CAPITAL LETTER A WITH GRAVE */
pub const XKB_KEY_Aacute                        :u32 = 0x00c1;  /* U+00C1 LATIN CAPITAL LETTER A WITH ACUTE */
pub const XKB_KEY_Acircumflex                   :u32 = 0x00c2;  /* U+00C2 LATIN CAPITAL LETTER A WITH CIRCUMFLEX */
pub const XKB_KEY_Atilde                        :u32 = 0x00c3;  /* U+00C3 LATIN CAPITAL LETTER A WITH TILDE */
pub const XKB_KEY_Adiaeresis                    :u32 = 0x00c4;  /* U+00C4 LATIN CAPITAL LETTER A WITH DIAERESIS */
pub const XKB_KEY_Aring                         :u32 = 0x00c5;  /* U+00C5 LATIN CAPITAL LETTER A WITH RING ABOVE */
pub const XKB_KEY_AE                            :u32 = 0x00c6;  /* U+00C6 LATIN CAPITAL LETTER AE */
pub const XKB_KEY_Ccedilla                      :u32 = 0x00c7;  /* U+00C7 LATIN CAPITAL LETTER C WITH CEDILLA */
pub const XKB_KEY_Egrave                        :u32 = 0x00c8;  /* U+00C8 LATIN CAPITAL LETTER E WITH GRAVE */
pub const XKB_KEY_Eacute                        :u32 = 0x00c9;  /* U+00C9 LATIN CAPITAL LETTER E WITH ACUTE */
pub const XKB_KEY_Ecircumflex                   :u32 = 0x00ca;  /* U+00CA LATIN CAPITAL LETTER E WITH CIRCUMFLEX */
pub const XKB_KEY_Ediaeresis                    :u32 = 0x00cb;  /* U+00CB LATIN CAPITAL LETTER E WITH DIAERESIS */
pub const XKB_KEY_Igrave                        :u32 = 0x00cc;  /* U+00CC LATIN CAPITAL LETTER I WITH GRAVE */
pub const XKB_KEY_Iacute                        :u32 = 0x00cd;  /* U+00CD LATIN CAPITAL LETTER I WITH ACUTE */
pub const XKB_KEY_Icircumflex                   :u32 = 0x00ce;  /* U+00CE LATIN CAPITAL LETTER I WITH CIRCUMFLEX */
pub const XKB_KEY_Idiaeresis                    :u32 = 0x00cf;  /* U+00CF LATIN CAPITAL LETTER I WITH DIAERESIS */
pub const XKB_KEY_ETH                           :u32 = 0x00d0;  /* U+00D0 LATIN CAPITAL LETTER ETH */
pub const XKB_KEY_Eth                           :u32 = 0x00d0;  /* deprecated */
pub const XKB_KEY_Ntilde                        :u32 = 0x00d1;  /* U+00D1 LATIN CAPITAL LETTER N WITH TILDE */
pub const XKB_KEY_Ograve                        :u32 = 0x00d2;  /* U+00D2 LATIN CAPITAL LETTER O WITH GRAVE */
pub const XKB_KEY_Oacute                        :u32 = 0x00d3;  /* U+00D3 LATIN CAPITAL LETTER O WITH ACUTE */
pub const XKB_KEY_Ocircumflex                   :u32 = 0x00d4;  /* U+00D4 LATIN CAPITAL LETTER O WITH CIRCUMFLEX */
pub const XKB_KEY_Otilde                        :u32 = 0x00d5;  /* U+00D5 LATIN CAPITAL LETTER O WITH TILDE */
pub const XKB_KEY_Odiaeresis                    :u32 = 0x00d6;  /* U+00D6 LATIN CAPITAL LETTER O WITH DIAERESIS */
pub const XKB_KEY_multiply                      :u32 = 0x00d7;  /* U+00D7 MULTIPLICATION SIGN */
pub const XKB_KEY_Oslash                        :u32 = 0x00d8;  /* U+00D8 LATIN CAPITAL LETTER O WITH STROKE */
pub const XKB_KEY_Ooblique                      :u32 = 0x00d8;  /* U+00D8 LATIN CAPITAL LETTER O WITH STROKE */
pub const XKB_KEY_Ugrave                        :u32 = 0x00d9;  /* U+00D9 LATIN CAPITAL LETTER U WITH GRAVE */
pub const XKB_KEY_Uacute                        :u32 = 0x00da;  /* U+00DA LATIN CAPITAL LETTER U WITH ACUTE */
pub const XKB_KEY_Ucircumflex                   :u32 = 0x00db;  /* U+00DB LATIN CAPITAL LETTER U WITH CIRCUMFLEX */
pub const XKB_KEY_Udiaeresis                    :u32 = 0x00dc;  /* U+00DC LATIN CAPITAL LETTER U WITH DIAERESIS */
pub const XKB_KEY_Yacute                        :u32 = 0x00dd;  /* U+00DD LATIN CAPITAL LETTER Y WITH ACUTE */
pub const XKB_KEY_THORN                         :u32 = 0x00de;  /* U+00DE LATIN CAPITAL LETTER THORN */
pub const XKB_KEY_Thorn                         :u32 = 0x00de;  /* deprecated */
pub const XKB_KEY_ssharp                        :u32 = 0x00df;  /* U+00DF LATIN SMALL LETTER SHARP S */
pub const XKB_KEY_agrave                        :u32 = 0x00e0;  /* U+00E0 LATIN SMALL LETTER A WITH GRAVE */
pub const XKB_KEY_aacute                        :u32 = 0x00e1;  /* U+00E1 LATIN SMALL LETTER A WITH ACUTE */
pub const XKB_KEY_acircumflex                   :u32 = 0x00e2;  /* U+00E2 LATIN SMALL LETTER A WITH CIRCUMFLEX */
pub const XKB_KEY_atilde                        :u32 = 0x00e3;  /* U+00E3 LATIN SMALL LETTER A WITH TILDE */
pub const XKB_KEY_adiaeresis                    :u32 = 0x00e4;  /* U+00E4 LATIN SMALL LETTER A WITH DIAERESIS */
pub const XKB_KEY_aring                         :u32 = 0x00e5;  /* U+00E5 LATIN SMALL LETTER A WITH RING ABOVE */
pub const XKB_KEY_ae                            :u32 = 0x00e6;  /* U+00E6 LATIN SMALL LETTER AE */
pub const XKB_KEY_ccedilla                      :u32 = 0x00e7;  /* U+00E7 LATIN SMALL LETTER C WITH CEDILLA */
pub const XKB_KEY_egrave                        :u32 = 0x00e8;  /* U+00E8 LATIN SMALL LETTER E WITH GRAVE */
pub const XKB_KEY_eacute                        :u32 = 0x00e9;  /* U+00E9 LATIN SMALL LETTER E WITH ACUTE */
pub const XKB_KEY_ecircumflex                   :u32 = 0x00ea;  /* U+00EA LATIN SMALL LETTER E WITH CIRCUMFLEX */
pub const XKB_KEY_ediaeresis                    :u32 = 0x00eb;  /* U+00EB LATIN SMALL LETTER E WITH DIAERESIS */
pub const XKB_KEY_igrave                        :u32 = 0x00ec;  /* U+00EC LATIN SMALL LETTER I WITH GRAVE */
pub const XKB_KEY_iacute                        :u32 = 0x00ed;  /* U+00ED LATIN SMALL LETTER I WITH ACUTE */
pub const XKB_KEY_icircumflex                   :u32 = 0x00ee;  /* U+00EE LATIN SMALL LETTER I WITH CIRCUMFLEX */
pub const XKB_KEY_idiaeresis                    :u32 = 0x00ef;  /* U+00EF LATIN SMALL LETTER I WITH DIAERESIS */
pub const XKB_KEY_eth                           :u32 = 0x00f0;  /* U+00F0 LATIN SMALL LETTER ETH */
pub const XKB_KEY_ntilde                        :u32 = 0x00f1;  /* U+00F1 LATIN SMALL LETTER N WITH TILDE */
pub const XKB_KEY_ograve                        :u32 = 0x00f2;  /* U+00F2 LATIN SMALL LETTER O WITH GRAVE */
pub const XKB_KEY_oacute                        :u32 = 0x00f3;  /* U+00F3 LATIN SMALL LETTER O WITH ACUTE */
pub const XKB_KEY_ocircumflex                   :u32 = 0x00f4;  /* U+00F4 LATIN SMALL LETTER O WITH CIRCUMFLEX */
pub const XKB_KEY_otilde                        :u32 = 0x00f5;  /* U+00F5 LATIN SMALL LETTER O WITH TILDE */
pub const XKB_KEY_odiaeresis                    :u32 = 0x00f6;  /* U+00F6 LATIN SMALL LETTER O WITH DIAERESIS */
pub const XKB_KEY_division                      :u32 = 0x00f7;  /* U+00F7 DIVISION SIGN */
pub const XKB_KEY_oslash                        :u32 = 0x00f8;  /* U+00F8 LATIN SMALL LETTER O WITH STROKE */
pub const XKB_KEY_ooblique                      :u32 = 0x00f8;  /* U+00F8 LATIN SMALL LETTER O WITH STROKE */
pub const XKB_KEY_ugrave                        :u32 = 0x00f9;  /* U+00F9 LATIN SMALL LETTER U WITH GRAVE */
pub const XKB_KEY_uacute                        :u32 = 0x00fa;  /* U+00FA LATIN SMALL LETTER U WITH ACUTE */
pub const XKB_KEY_ucircumflex                   :u32 = 0x00fb;  /* U+00FB LATIN SMALL LETTER U WITH CIRCUMFLEX */
pub const XKB_KEY_udiaeresis                    :u32 = 0x00fc;  /* U+00FC LATIN SMALL LETTER U WITH DIAERESIS */
pub const XKB_KEY_yacute                        :u32 = 0x00fd;  /* U+00FD LATIN SMALL LETTER Y WITH ACUTE */
pub const XKB_KEY_thorn                         :u32 = 0x00fe;  /* U+00FE LATIN SMALL LETTER THORN */
pub const XKB_KEY_ydiaeresis                    :u32 = 0x00ff;  /* U+00FF LATIN SMALL LETTER Y WITH DIAERESIS */

/*
 * Latin 2
 * Byte 3 = 1
 */

pub const XKB_KEY_Aogonek                       :u32 = 0x01a1;  /* U+0104 LATIN CAPITAL LETTER A WITH OGONEK */
pub const XKB_KEY_breve                         :u32 = 0x01a2;  /* U+02D8 BREVE */
pub const XKB_KEY_Lstroke                       :u32 = 0x01a3;  /* U+0141 LATIN CAPITAL LETTER L WITH STROKE */
pub const XKB_KEY_Lcaron                        :u32 = 0x01a5;  /* U+013D LATIN CAPITAL LETTER L WITH CARON */
pub const XKB_KEY_Sacute                        :u32 = 0x01a6;  /* U+015A LATIN CAPITAL LETTER S WITH ACUTE */
pub const XKB_KEY_Scaron                        :u32 = 0x01a9;  /* U+0160 LATIN CAPITAL LETTER S WITH CARON */
pub const XKB_KEY_Scedilla                      :u32 = 0x01aa;  /* U+015E LATIN CAPITAL LETTER S WITH CEDILLA */
pub const XKB_KEY_Tcaron                        :u32 = 0x01ab;  /* U+0164 LATIN CAPITAL LETTER T WITH CARON */
pub const XKB_KEY_Zacute                        :u32 = 0x01ac;  /* U+0179 LATIN CAPITAL LETTER Z WITH ACUTE */
pub const XKB_KEY_Zcaron                        :u32 = 0x01ae;  /* U+017D LATIN CAPITAL LETTER Z WITH CARON */
pub const XKB_KEY_Zabovedot                     :u32 = 0x01af;  /* U+017B LATIN CAPITAL LETTER Z WITH DOT ABOVE */
pub const XKB_KEY_aogonek                       :u32 = 0x01b1;  /* U+0105 LATIN SMALL LETTER A WITH OGONEK */
pub const XKB_KEY_ogonek                        :u32 = 0x01b2;  /* U+02DB OGONEK */
pub const XKB_KEY_lstroke                       :u32 = 0x01b3;  /* U+0142 LATIN SMALL LETTER L WITH STROKE */
pub const XKB_KEY_lcaron                        :u32 = 0x01b5;  /* U+013E LATIN SMALL LETTER L WITH CARON */
pub const XKB_KEY_sacute                        :u32 = 0x01b6;  /* U+015B LATIN SMALL LETTER S WITH ACUTE */
pub const XKB_KEY_caron                         :u32 = 0x01b7;  /* U+02C7 CARON */
pub const XKB_KEY_scaron                        :u32 = 0x01b9;  /* U+0161 LATIN SMALL LETTER S WITH CARON */
pub const XKB_KEY_scedilla                      :u32 = 0x01ba;  /* U+015F LATIN SMALL LETTER S WITH CEDILLA */
pub const XKB_KEY_tcaron                        :u32 = 0x01bb;  /* U+0165 LATIN SMALL LETTER T WITH CARON */
pub const XKB_KEY_zacute                        :u32 = 0x01bc;  /* U+017A LATIN SMALL LETTER Z WITH ACUTE */
pub const XKB_KEY_doubleacute                   :u32 = 0x01bd;  /* U+02DD DOUBLE ACUTE ACCENT */
pub const XKB_KEY_zcaron                        :u32 = 0x01be;  /* U+017E LATIN SMALL LETTER Z WITH CARON */
pub const XKB_KEY_zabovedot                     :u32 = 0x01bf;  /* U+017C LATIN SMALL LETTER Z WITH DOT ABOVE */
pub const XKB_KEY_Racute                        :u32 = 0x01c0;  /* U+0154 LATIN CAPITAL LETTER R WITH ACUTE */
pub const XKB_KEY_Abreve                        :u32 = 0x01c3;  /* U+0102 LATIN CAPITAL LETTER A WITH BREVE */
pub const XKB_KEY_Lacute                        :u32 = 0x01c5;  /* U+0139 LATIN CAPITAL LETTER L WITH ACUTE */
pub const XKB_KEY_Cacute                        :u32 = 0x01c6;  /* U+0106 LATIN CAPITAL LETTER C WITH ACUTE */
pub const XKB_KEY_Ccaron                        :u32 = 0x01c8;  /* U+010C LATIN CAPITAL LETTER C WITH CARON */
pub const XKB_KEY_Eogonek                       :u32 = 0x01ca;  /* U+0118 LATIN CAPITAL LETTER E WITH OGONEK */
pub const XKB_KEY_Ecaron                        :u32 = 0x01cc;  /* U+011A LATIN CAPITAL LETTER E WITH CARON */
pub const XKB_KEY_Dcaron                        :u32 = 0x01cf;  /* U+010E LATIN CAPITAL LETTER D WITH CARON */
pub const XKB_KEY_Dstroke                       :u32 = 0x01d0;  /* U+0110 LATIN CAPITAL LETTER D WITH STROKE */
pub const XKB_KEY_Nacute                        :u32 = 0x01d1;  /* U+0143 LATIN CAPITAL LETTER N WITH ACUTE */
pub const XKB_KEY_Ncaron                        :u32 = 0x01d2;  /* U+0147 LATIN CAPITAL LETTER N WITH CARON */
pub const XKB_KEY_Odoubleacute                  :u32 = 0x01d5;  /* U+0150 LATIN CAPITAL LETTER O WITH DOUBLE ACUTE */
pub const XKB_KEY_Rcaron                        :u32 = 0x01d8;  /* U+0158 LATIN CAPITAL LETTER R WITH CARON */
pub const XKB_KEY_Uring                         :u32 = 0x01d9;  /* U+016E LATIN CAPITAL LETTER U WITH RING ABOVE */
pub const XKB_KEY_Udoubleacute                  :u32 = 0x01db;  /* U+0170 LATIN CAPITAL LETTER U WITH DOUBLE ACUTE */
pub const XKB_KEY_Tcedilla                      :u32 = 0x01de;  /* U+0162 LATIN CAPITAL LETTER T WITH CEDILLA */
pub const XKB_KEY_racute                        :u32 = 0x01e0;  /* U+0155 LATIN SMALL LETTER R WITH ACUTE */
pub const XKB_KEY_abreve                        :u32 = 0x01e3;  /* U+0103 LATIN SMALL LETTER A WITH BREVE */
pub const XKB_KEY_lacute                        :u32 = 0x01e5;  /* U+013A LATIN SMALL LETTER L WITH ACUTE */
pub const XKB_KEY_cacute                        :u32 = 0x01e6;  /* U+0107 LATIN SMALL LETTER C WITH ACUTE */
pub const XKB_KEY_ccaron                        :u32 = 0x01e8;  /* U+010D LATIN SMALL LETTER C WITH CARON */
pub const XKB_KEY_eogonek                       :u32 = 0x01ea;  /* U+0119 LATIN SMALL LETTER E WITH OGONEK */
pub const XKB_KEY_ecaron                        :u32 = 0x01ec;  /* U+011B LATIN SMALL LETTER E WITH CARON */
pub const XKB_KEY_dcaron                        :u32 = 0x01ef;  /* U+010F LATIN SMALL LETTER D WITH CARON */
pub const XKB_KEY_dstroke                       :u32 = 0x01f0;  /* U+0111 LATIN SMALL LETTER D WITH STROKE */
pub const XKB_KEY_nacute                        :u32 = 0x01f1;  /* U+0144 LATIN SMALL LETTER N WITH ACUTE */
pub const XKB_KEY_ncaron                        :u32 = 0x01f2;  /* U+0148 LATIN SMALL LETTER N WITH CARON */
pub const XKB_KEY_odoubleacute                  :u32 = 0x01f5;  /* U+0151 LATIN SMALL LETTER O WITH DOUBLE ACUTE */
pub const XKB_KEY_rcaron                        :u32 = 0x01f8;  /* U+0159 LATIN SMALL LETTER R WITH CARON */
pub const XKB_KEY_uring                         :u32 = 0x01f9;  /* U+016F LATIN SMALL LETTER U WITH RING ABOVE */
pub const XKB_KEY_udoubleacute                  :u32 = 0x01fb;  /* U+0171 LATIN SMALL LETTER U WITH DOUBLE ACUTE */
pub const XKB_KEY_tcedilla                      :u32 = 0x01fe;  /* U+0163 LATIN SMALL LETTER T WITH CEDILLA */
pub const XKB_KEY_abovedot                      :u32 = 0x01ff;  /* U+02D9 DOT ABOVE */

/*
 * Latin 3
 * Byte 3 = 2
 */

pub const XKB_KEY_Hstroke                       :u32 = 0x02a1;  /* U+0126 LATIN CAPITAL LETTER H WITH STROKE */
pub const XKB_KEY_Hcircumflex                   :u32 = 0x02a6;  /* U+0124 LATIN CAPITAL LETTER H WITH CIRCUMFLEX */
pub const XKB_KEY_Iabovedot                     :u32 = 0x02a9;  /* U+0130 LATIN CAPITAL LETTER I WITH DOT ABOVE */
pub const XKB_KEY_Gbreve                        :u32 = 0x02ab;  /* U+011E LATIN CAPITAL LETTER G WITH BREVE */
pub const XKB_KEY_Jcircumflex                   :u32 = 0x02ac;  /* U+0134 LATIN CAPITAL LETTER J WITH CIRCUMFLEX */
pub const XKB_KEY_hstroke                       :u32 = 0x02b1;  /* U+0127 LATIN SMALL LETTER H WITH STROKE */
pub const XKB_KEY_hcircumflex                   :u32 = 0x02b6;  /* U+0125 LATIN SMALL LETTER H WITH CIRCUMFLEX */
pub const XKB_KEY_idotless                      :u32 = 0x02b9;  /* U+0131 LATIN SMALL LETTER DOTLESS I */
pub const XKB_KEY_gbreve                        :u32 = 0x02bb;  /* U+011F LATIN SMALL LETTER G WITH BREVE */
pub const XKB_KEY_jcircumflex                   :u32 = 0x02bc;  /* U+0135 LATIN SMALL LETTER J WITH CIRCUMFLEX */
pub const XKB_KEY_Cabovedot                     :u32 = 0x02c5;  /* U+010A LATIN CAPITAL LETTER C WITH DOT ABOVE */
pub const XKB_KEY_Ccircumflex                   :u32 = 0x02c6;  /* U+0108 LATIN CAPITAL LETTER C WITH CIRCUMFLEX */
pub const XKB_KEY_Gabovedot                     :u32 = 0x02d5;  /* U+0120 LATIN CAPITAL LETTER G WITH DOT ABOVE */
pub const XKB_KEY_Gcircumflex                   :u32 = 0x02d8;  /* U+011C LATIN CAPITAL LETTER G WITH CIRCUMFLEX */
pub const XKB_KEY_Ubreve                        :u32 = 0x02dd;  /* U+016C LATIN CAPITAL LETTER U WITH BREVE */
pub const XKB_KEY_Scircumflex                   :u32 = 0x02de;  /* U+015C LATIN CAPITAL LETTER S WITH CIRCUMFLEX */
pub const XKB_KEY_cabovedot                     :u32 = 0x02e5;  /* U+010B LATIN SMALL LETTER C WITH DOT ABOVE */
pub const XKB_KEY_ccircumflex                   :u32 = 0x02e6;  /* U+0109 LATIN SMALL LETTER C WITH CIRCUMFLEX */
pub const XKB_KEY_gabovedot                     :u32 = 0x02f5;  /* U+0121 LATIN SMALL LETTER G WITH DOT ABOVE */
pub const XKB_KEY_gcircumflex                   :u32 = 0x02f8;  /* U+011D LATIN SMALL LETTER G WITH CIRCUMFLEX */
pub const XKB_KEY_ubreve                        :u32 = 0x02fd;  /* U+016D LATIN SMALL LETTER U WITH BREVE */
pub const XKB_KEY_scircumflex                   :u32 = 0x02fe;  /* U+015D LATIN SMALL LETTER S WITH CIRCUMFLEX */


/*
 * Latin 4
 * Byte 3 = 3
 */

pub const XKB_KEY_kra                           :u32 = 0x03a2;  /* U+0138 LATIN SMALL LETTER KRA */
pub const XKB_KEY_kappa                         :u32 = 0x03a2;  /* deprecated */
pub const XKB_KEY_Rcedilla                      :u32 = 0x03a3;  /* U+0156 LATIN CAPITAL LETTER R WITH CEDILLA */
pub const XKB_KEY_Itilde                        :u32 = 0x03a5;  /* U+0128 LATIN CAPITAL LETTER I WITH TILDE */
pub const XKB_KEY_Lcedilla                      :u32 = 0x03a6;  /* U+013B LATIN CAPITAL LETTER L WITH CEDILLA */
pub const XKB_KEY_Emacron                       :u32 = 0x03aa;  /* U+0112 LATIN CAPITAL LETTER E WITH MACRON */
pub const XKB_KEY_Gcedilla                      :u32 = 0x03ab;  /* U+0122 LATIN CAPITAL LETTER G WITH CEDILLA */
pub const XKB_KEY_Tslash                        :u32 = 0x03ac;  /* U+0166 LATIN CAPITAL LETTER T WITH STROKE */
pub const XKB_KEY_rcedilla                      :u32 = 0x03b3;  /* U+0157 LATIN SMALL LETTER R WITH CEDILLA */
pub const XKB_KEY_itilde                        :u32 = 0x03b5;  /* U+0129 LATIN SMALL LETTER I WITH TILDE */
pub const XKB_KEY_lcedilla                      :u32 = 0x03b6;  /* U+013C LATIN SMALL LETTER L WITH CEDILLA */
pub const XKB_KEY_emacron                       :u32 = 0x03ba;  /* U+0113 LATIN SMALL LETTER E WITH MACRON */
pub const XKB_KEY_gcedilla                      :u32 = 0x03bb;  /* U+0123 LATIN SMALL LETTER G WITH CEDILLA */
pub const XKB_KEY_tslash                        :u32 = 0x03bc;  /* U+0167 LATIN SMALL LETTER T WITH STROKE */
pub const XKB_KEY_ENG                           :u32 = 0x03bd;  /* U+014A LATIN CAPITAL LETTER ENG */
pub const XKB_KEY_eng                           :u32 = 0x03bf;  /* U+014B LATIN SMALL LETTER ENG */
pub const XKB_KEY_Amacron                       :u32 = 0x03c0;  /* U+0100 LATIN CAPITAL LETTER A WITH MACRON */
pub const XKB_KEY_Iogonek                       :u32 = 0x03c7;  /* U+012E LATIN CAPITAL LETTER I WITH OGONEK */
pub const XKB_KEY_Eabovedot                     :u32 = 0x03cc;  /* U+0116 LATIN CAPITAL LETTER E WITH DOT ABOVE */
pub const XKB_KEY_Imacron                       :u32 = 0x03cf;  /* U+012A LATIN CAPITAL LETTER I WITH MACRON */
pub const XKB_KEY_Ncedilla                      :u32 = 0x03d1;  /* U+0145 LATIN CAPITAL LETTER N WITH CEDILLA */
pub const XKB_KEY_Omacron                       :u32 = 0x03d2;  /* U+014C LATIN CAPITAL LETTER O WITH MACRON */
pub const XKB_KEY_Kcedilla                      :u32 = 0x03d3;  /* U+0136 LATIN CAPITAL LETTER K WITH CEDILLA */
pub const XKB_KEY_Uogonek                       :u32 = 0x03d9;  /* U+0172 LATIN CAPITAL LETTER U WITH OGONEK */
pub const XKB_KEY_Utilde                        :u32 = 0x03dd;  /* U+0168 LATIN CAPITAL LETTER U WITH TILDE */
pub const XKB_KEY_Umacron                       :u32 = 0x03de;  /* U+016A LATIN CAPITAL LETTER U WITH MACRON */
pub const XKB_KEY_amacron                       :u32 = 0x03e0;  /* U+0101 LATIN SMALL LETTER A WITH MACRON */
pub const XKB_KEY_iogonek                       :u32 = 0x03e7;  /* U+012F LATIN SMALL LETTER I WITH OGONEK */
pub const XKB_KEY_eabovedot                     :u32 = 0x03ec;  /* U+0117 LATIN SMALL LETTER E WITH DOT ABOVE */
pub const XKB_KEY_imacron                       :u32 = 0x03ef;  /* U+012B LATIN SMALL LETTER I WITH MACRON */
pub const XKB_KEY_ncedilla                      :u32 = 0x03f1;  /* U+0146 LATIN SMALL LETTER N WITH CEDILLA */
pub const XKB_KEY_omacron                       :u32 = 0x03f2;  /* U+014D LATIN SMALL LETTER O WITH MACRON */
pub const XKB_KEY_kcedilla                      :u32 = 0x03f3;  /* U+0137 LATIN SMALL LETTER K WITH CEDILLA */
pub const XKB_KEY_uogonek                       :u32 = 0x03f9;  /* U+0173 LATIN SMALL LETTER U WITH OGONEK */
pub const XKB_KEY_utilde                        :u32 = 0x03fd;  /* U+0169 LATIN SMALL LETTER U WITH TILDE */
pub const XKB_KEY_umacron                       :u32 = 0x03fe;  /* U+016B LATIN SMALL LETTER U WITH MACRON */

/*
 * Latin 8
 */
pub const XKB_KEY_Wcircumflex                :u32 = 0x1000174;  /* U+0174 LATIN CAPITAL LETTER W WITH CIRCUMFLEX */
pub const XKB_KEY_wcircumflex                :u32 = 0x1000175;  /* U+0175 LATIN SMALL LETTER W WITH CIRCUMFLEX */
pub const XKB_KEY_Ycircumflex                :u32 = 0x1000176;  /* U+0176 LATIN CAPITAL LETTER Y WITH CIRCUMFLEX */
pub const XKB_KEY_ycircumflex                :u32 = 0x1000177;  /* U+0177 LATIN SMALL LETTER Y WITH CIRCUMFLEX */
pub const XKB_KEY_Babovedot                  :u32 = 0x1001e02;  /* U+1E02 LATIN CAPITAL LETTER B WITH DOT ABOVE */
pub const XKB_KEY_babovedot                  :u32 = 0x1001e03;  /* U+1E03 LATIN SMALL LETTER B WITH DOT ABOVE */
pub const XKB_KEY_Dabovedot                  :u32 = 0x1001e0a;  /* U+1E0A LATIN CAPITAL LETTER D WITH DOT ABOVE */
pub const XKB_KEY_dabovedot                  :u32 = 0x1001e0b;  /* U+1E0B LATIN SMALL LETTER D WITH DOT ABOVE */
pub const XKB_KEY_Fabovedot                  :u32 = 0x1001e1e;  /* U+1E1E LATIN CAPITAL LETTER F WITH DOT ABOVE */
pub const XKB_KEY_fabovedot                  :u32 = 0x1001e1f;  /* U+1E1F LATIN SMALL LETTER F WITH DOT ABOVE */
pub const XKB_KEY_Mabovedot                  :u32 = 0x1001e40;  /* U+1E40 LATIN CAPITAL LETTER M WITH DOT ABOVE */
pub const XKB_KEY_mabovedot                  :u32 = 0x1001e41;  /* U+1E41 LATIN SMALL LETTER M WITH DOT ABOVE */
pub const XKB_KEY_Pabovedot                  :u32 = 0x1001e56;  /* U+1E56 LATIN CAPITAL LETTER P WITH DOT ABOVE */
pub const XKB_KEY_pabovedot                  :u32 = 0x1001e57;  /* U+1E57 LATIN SMALL LETTER P WITH DOT ABOVE */
pub const XKB_KEY_Sabovedot                  :u32 = 0x1001e60;  /* U+1E60 LATIN CAPITAL LETTER S WITH DOT ABOVE */
pub const XKB_KEY_sabovedot                  :u32 = 0x1001e61;  /* U+1E61 LATIN SMALL LETTER S WITH DOT ABOVE */
pub const XKB_KEY_Tabovedot                  :u32 = 0x1001e6a;  /* U+1E6A LATIN CAPITAL LETTER T WITH DOT ABOVE */
pub const XKB_KEY_tabovedot                  :u32 = 0x1001e6b;  /* U+1E6B LATIN SMALL LETTER T WITH DOT ABOVE */
pub const XKB_KEY_Wgrave                     :u32 = 0x1001e80;  /* U+1E80 LATIN CAPITAL LETTER W WITH GRAVE */
pub const XKB_KEY_wgrave                     :u32 = 0x1001e81;  /* U+1E81 LATIN SMALL LETTER W WITH GRAVE */
pub const XKB_KEY_Wacute                     :u32 = 0x1001e82;  /* U+1E82 LATIN CAPITAL LETTER W WITH ACUTE */
pub const XKB_KEY_wacute                     :u32 = 0x1001e83;  /* U+1E83 LATIN SMALL LETTER W WITH ACUTE */
pub const XKB_KEY_Wdiaeresis                 :u32 = 0x1001e84;  /* U+1E84 LATIN CAPITAL LETTER W WITH DIAERESIS */
pub const XKB_KEY_wdiaeresis                 :u32 = 0x1001e85;  /* U+1E85 LATIN SMALL LETTER W WITH DIAERESIS */
pub const XKB_KEY_Ygrave                     :u32 = 0x1001ef2;  /* U+1EF2 LATIN CAPITAL LETTER Y WITH GRAVE */
pub const XKB_KEY_ygrave                     :u32 = 0x1001ef3;  /* U+1EF3 LATIN SMALL LETTER Y WITH GRAVE */

/*
 * Latin 9
 * Byte 3 = :u32 = 0x13;
 */

pub const XKB_KEY_OE                            :u32 = 0x13bc;  /* U+0152 LATIN CAPITAL LIGATURE OE */
pub const XKB_KEY_oe                            :u32 = 0x13bd;  /* U+0153 LATIN SMALL LIGATURE OE */
pub const XKB_KEY_Ydiaeresis                    :u32 = 0x13be;  /* U+0178 LATIN CAPITAL LETTER Y WITH DIAERESIS */

/*
 * Katakana
 * Byte 3 = 4
 */

pub const XKB_KEY_overline                      :u32 = 0x047e;  /* U+203E OVERLINE */
pub const XKB_KEY_kana_fullstop                 :u32 = 0x04a1;  /* U+3002 IDEOGRAPHIC FULL STOP */
pub const XKB_KEY_kana_openingbracket           :u32 = 0x04a2;  /* U+300C LEFT CORNER BRACKET */
pub const XKB_KEY_kana_closingbracket           :u32 = 0x04a3;  /* U+300D RIGHT CORNER BRACKET */
pub const XKB_KEY_kana_comma                    :u32 = 0x04a4;  /* U+3001 IDEOGRAPHIC COMMA */
pub const XKB_KEY_kana_conjunctive              :u32 = 0x04a5;  /* U+30FB KATAKANA MIDDLE DOT */
pub const XKB_KEY_kana_middledot                :u32 = 0x04a5;  /* deprecated */
pub const XKB_KEY_kana_WO                       :u32 = 0x04a6;  /* U+30F2 KATAKANA LETTER WO */
pub const XKB_KEY_kana_a                        :u32 = 0x04a7;  /* U+30A1 KATAKANA LETTER SMALL A */
pub const XKB_KEY_kana_i                        :u32 = 0x04a8;  /* U+30A3 KATAKANA LETTER SMALL I */
pub const XKB_KEY_kana_u                        :u32 = 0x04a9;  /* U+30A5 KATAKANA LETTER SMALL U */
pub const XKB_KEY_kana_e                        :u32 = 0x04aa;  /* U+30A7 KATAKANA LETTER SMALL E */
pub const XKB_KEY_kana_o                        :u32 = 0x04ab;  /* U+30A9 KATAKANA LETTER SMALL O */
pub const XKB_KEY_kana_ya                       :u32 = 0x04ac;  /* U+30E3 KATAKANA LETTER SMALL YA */
pub const XKB_KEY_kana_yu                       :u32 = 0x04ad;  /* U+30E5 KATAKANA LETTER SMALL YU */
pub const XKB_KEY_kana_yo                       :u32 = 0x04ae;  /* U+30E7 KATAKANA LETTER SMALL YO */
pub const XKB_KEY_kana_tsu                      :u32 = 0x04af;  /* U+30C3 KATAKANA LETTER SMALL TU */
pub const XKB_KEY_kana_tu                       :u32 = 0x04af;  /* deprecated */
pub const XKB_KEY_prolongedsound                :u32 = 0x04b0;  /* U+30FC KATAKANA-HIRAGANA PROLONGED SOUND MARK */
pub const XKB_KEY_kana_A                        :u32 = 0x04b1;  /* U+30A2 KATAKANA LETTER A */
pub const XKB_KEY_kana_I                        :u32 = 0x04b2;  /* U+30A4 KATAKANA LETTER I */
pub const XKB_KEY_kana_U                        :u32 = 0x04b3;  /* U+30A6 KATAKANA LETTER U */
pub const XKB_KEY_kana_E                        :u32 = 0x04b4;  /* U+30A8 KATAKANA LETTER E */
pub const XKB_KEY_kana_O                        :u32 = 0x04b5;  /* U+30AA KATAKANA LETTER O */
pub const XKB_KEY_kana_KA                       :u32 = 0x04b6;  /* U+30AB KATAKANA LETTER KA */
pub const XKB_KEY_kana_KI                       :u32 = 0x04b7;  /* U+30AD KATAKANA LETTER KI */
pub const XKB_KEY_kana_KU                       :u32 = 0x04b8;  /* U+30AF KATAKANA LETTER KU */
pub const XKB_KEY_kana_KE                       :u32 = 0x04b9;  /* U+30B1 KATAKANA LETTER KE */
pub const XKB_KEY_kana_KO                       :u32 = 0x04ba;  /* U+30B3 KATAKANA LETTER KO */
pub const XKB_KEY_kana_SA                       :u32 = 0x04bb;  /* U+30B5 KATAKANA LETTER SA */
pub const XKB_KEY_kana_SHI                      :u32 = 0x04bc;  /* U+30B7 KATAKANA LETTER SI */
pub const XKB_KEY_kana_SU                       :u32 = 0x04bd;  /* U+30B9 KATAKANA LETTER SU */
pub const XKB_KEY_kana_SE                       :u32 = 0x04be;  /* U+30BB KATAKANA LETTER SE */
pub const XKB_KEY_kana_SO                       :u32 = 0x04bf;  /* U+30BD KATAKANA LETTER SO */
pub const XKB_KEY_kana_TA                       :u32 = 0x04c0;  /* U+30BF KATAKANA LETTER TA */
pub const XKB_KEY_kana_CHI                      :u32 = 0x04c1;  /* U+30C1 KATAKANA LETTER TI */
pub const XKB_KEY_kana_TI                       :u32 = 0x04c1;  /* deprecated */
pub const XKB_KEY_kana_TSU                      :u32 = 0x04c2;  /* U+30C4 KATAKANA LETTER TU */
pub const XKB_KEY_kana_TU                       :u32 = 0x04c2;  /* deprecated */
pub const XKB_KEY_kana_TE                       :u32 = 0x04c3;  /* U+30C6 KATAKANA LETTER TE */
pub const XKB_KEY_kana_TO                       :u32 = 0x04c4;  /* U+30C8 KATAKANA LETTER TO */
pub const XKB_KEY_kana_NA                       :u32 = 0x04c5;  /* U+30CA KATAKANA LETTER NA */
pub const XKB_KEY_kana_NI                       :u32 = 0x04c6;  /* U+30CB KATAKANA LETTER NI */
pub const XKB_KEY_kana_NU                       :u32 = 0x04c7;  /* U+30CC KATAKANA LETTER NU */
pub const XKB_KEY_kana_NE                       :u32 = 0x04c8;  /* U+30CD KATAKANA LETTER NE */
pub const XKB_KEY_kana_NO                       :u32 = 0x04c9;  /* U+30CE KATAKANA LETTER NO */
pub const XKB_KEY_kana_HA                       :u32 = 0x04ca;  /* U+30CF KATAKANA LETTER HA */
pub const XKB_KEY_kana_HI                       :u32 = 0x04cb;  /* U+30D2 KATAKANA LETTER HI */
pub const XKB_KEY_kana_FU                       :u32 = 0x04cc;  /* U+30D5 KATAKANA LETTER HU */
pub const XKB_KEY_kana_HU                       :u32 = 0x04cc;  /* deprecated */
pub const XKB_KEY_kana_HE                       :u32 = 0x04cd;  /* U+30D8 KATAKANA LETTER HE */
pub const XKB_KEY_kana_HO                       :u32 = 0x04ce;  /* U+30DB KATAKANA LETTER HO */
pub const XKB_KEY_kana_MA                       :u32 = 0x04cf;  /* U+30DE KATAKANA LETTER MA */
pub const XKB_KEY_kana_MI                       :u32 = 0x04d0;  /* U+30DF KATAKANA LETTER MI */
pub const XKB_KEY_kana_MU                       :u32 = 0x04d1;  /* U+30E0 KATAKANA LETTER MU */
pub const XKB_KEY_kana_ME                       :u32 = 0x04d2;  /* U+30E1 KATAKANA LETTER ME */
pub const XKB_KEY_kana_MO                       :u32 = 0x04d3;  /* U+30E2 KATAKANA LETTER MO */
pub const XKB_KEY_kana_YA                       :u32 = 0x04d4;  /* U+30E4 KATAKANA LETTER YA */
pub const XKB_KEY_kana_YU                       :u32 = 0x04d5;  /* U+30E6 KATAKANA LETTER YU */
pub const XKB_KEY_kana_YO                       :u32 = 0x04d6;  /* U+30E8 KATAKANA LETTER YO */
pub const XKB_KEY_kana_RA                       :u32 = 0x04d7;  /* U+30E9 KATAKANA LETTER RA */
pub const XKB_KEY_kana_RI                       :u32 = 0x04d8;  /* U+30EA KATAKANA LETTER RI */
pub const XKB_KEY_kana_RU                       :u32 = 0x04d9;  /* U+30EB KATAKANA LETTER RU */
pub const XKB_KEY_kana_RE                       :u32 = 0x04da;  /* U+30EC KATAKANA LETTER RE */
pub const XKB_KEY_kana_RO                       :u32 = 0x04db;  /* U+30ED KATAKANA LETTER RO */
pub const XKB_KEY_kana_WA                       :u32 = 0x04dc;  /* U+30EF KATAKANA LETTER WA */
pub const XKB_KEY_kana_N                        :u32 = 0x04dd;  /* U+30F3 KATAKANA LETTER N */
pub const XKB_KEY_voicedsound                   :u32 = 0x04de;  /* U+309B KATAKANA-HIRAGANA VOICED SOUND MARK */
pub const XKB_KEY_semivoicedsound               :u32 = 0x04df;  /* U+309C KATAKANA-HIRAGANA SEMI-VOICED SOUND MARK */
pub const XKB_KEY_kana_switch                   :u32 = 0xff7e;  /* Alias for mode_switch */

/*
 * Arabic
 * Byte 3 = 5
 */

pub const XKB_KEY_Farsi_0                    :u32 = 0x10006f0;  /* U+06F0 EXTENDED ARABIC-INDIC DIGIT ZERO */
pub const XKB_KEY_Farsi_1                    :u32 = 0x10006f1;  /* U+06F1 EXTENDED ARABIC-INDIC DIGIT ONE */
pub const XKB_KEY_Farsi_2                    :u32 = 0x10006f2;  /* U+06F2 EXTENDED ARABIC-INDIC DIGIT TWO */
pub const XKB_KEY_Farsi_3                    :u32 = 0x10006f3;  /* U+06F3 EXTENDED ARABIC-INDIC DIGIT THREE */
pub const XKB_KEY_Farsi_4                    :u32 = 0x10006f4;  /* U+06F4 EXTENDED ARABIC-INDIC DIGIT FOUR */
pub const XKB_KEY_Farsi_5                    :u32 = 0x10006f5;  /* U+06F5 EXTENDED ARABIC-INDIC DIGIT FIVE */
pub const XKB_KEY_Farsi_6                    :u32 = 0x10006f6;  /* U+06F6 EXTENDED ARABIC-INDIC DIGIT SIX */
pub const XKB_KEY_Farsi_7                    :u32 = 0x10006f7;  /* U+06F7 EXTENDED ARABIC-INDIC DIGIT SEVEN */
pub const XKB_KEY_Farsi_8                    :u32 = 0x10006f8;  /* U+06F8 EXTENDED ARABIC-INDIC DIGIT EIGHT */
pub const XKB_KEY_Farsi_9                    :u32 = 0x10006f9;  /* U+06F9 EXTENDED ARABIC-INDIC DIGIT NINE */
pub const XKB_KEY_Arabic_percent             :u32 = 0x100066a;  /* U+066A ARABIC PERCENT SIGN */
pub const XKB_KEY_Arabic_superscript_alef    :u32 = 0x1000670;  /* U+0670 ARABIC LETTER SUPERSCRIPT ALEF */
pub const XKB_KEY_Arabic_tteh                :u32 = 0x1000679;  /* U+0679 ARABIC LETTER TTEH */
pub const XKB_KEY_Arabic_peh                 :u32 = 0x100067e;  /* U+067E ARABIC LETTER PEH */
pub const XKB_KEY_Arabic_tcheh               :u32 = 0x1000686;  /* U+0686 ARABIC LETTER TCHEH */
pub const XKB_KEY_Arabic_ddal                :u32 = 0x1000688;  /* U+0688 ARABIC LETTER DDAL */
pub const XKB_KEY_Arabic_rreh                :u32 = 0x1000691;  /* U+0691 ARABIC LETTER RREH */
pub const XKB_KEY_Arabic_comma                  :u32 = 0x05ac;  /* U+060C ARABIC COMMA */
pub const XKB_KEY_Arabic_fullstop            :u32 = 0x10006d4;  /* U+06D4 ARABIC FULL STOP */
pub const XKB_KEY_Arabic_0                   :u32 = 0x1000660;  /* U+0660 ARABIC-INDIC DIGIT ZERO */
pub const XKB_KEY_Arabic_1                   :u32 = 0x1000661;  /* U+0661 ARABIC-INDIC DIGIT ONE */
pub const XKB_KEY_Arabic_2                   :u32 = 0x1000662;  /* U+0662 ARABIC-INDIC DIGIT TWO */
pub const XKB_KEY_Arabic_3                   :u32 = 0x1000663;  /* U+0663 ARABIC-INDIC DIGIT THREE */
pub const XKB_KEY_Arabic_4                   :u32 = 0x1000664;  /* U+0664 ARABIC-INDIC DIGIT FOUR */
pub const XKB_KEY_Arabic_5                   :u32 = 0x1000665;  /* U+0665 ARABIC-INDIC DIGIT FIVE */
pub const XKB_KEY_Arabic_6                   :u32 = 0x1000666;  /* U+0666 ARABIC-INDIC DIGIT SIX */
pub const XKB_KEY_Arabic_7                   :u32 = 0x1000667;  /* U+0667 ARABIC-INDIC DIGIT SEVEN */
pub const XKB_KEY_Arabic_8                   :u32 = 0x1000668;  /* U+0668 ARABIC-INDIC DIGIT EIGHT */
pub const XKB_KEY_Arabic_9                   :u32 = 0x1000669;  /* U+0669 ARABIC-INDIC DIGIT NINE */
pub const XKB_KEY_Arabic_semicolon              :u32 = 0x05bb;  /* U+061B ARABIC SEMICOLON */
pub const XKB_KEY_Arabic_question_mark          :u32 = 0x05bf;  /* U+061F ARABIC QUESTION MARK */
pub const XKB_KEY_Arabic_hamza                  :u32 = 0x05c1;  /* U+0621 ARABIC LETTER HAMZA */
pub const XKB_KEY_Arabic_maddaonalef            :u32 = 0x05c2;  /* U+0622 ARABIC LETTER ALEF WITH MADDA ABOVE */
pub const XKB_KEY_Arabic_hamzaonalef            :u32 = 0x05c3;  /* U+0623 ARABIC LETTER ALEF WITH HAMZA ABOVE */
pub const XKB_KEY_Arabic_hamzaonwaw             :u32 = 0x05c4;  /* U+0624 ARABIC LETTER WAW WITH HAMZA ABOVE */
pub const XKB_KEY_Arabic_hamzaunderalef         :u32 = 0x05c5;  /* U+0625 ARABIC LETTER ALEF WITH HAMZA BELOW */
pub const XKB_KEY_Arabic_hamzaonyeh             :u32 = 0x05c6;  /* U+0626 ARABIC LETTER YEH WITH HAMZA ABOVE */
pub const XKB_KEY_Arabic_alef                   :u32 = 0x05c7;  /* U+0627 ARABIC LETTER ALEF */
pub const XKB_KEY_Arabic_beh                    :u32 = 0x05c8;  /* U+0628 ARABIC LETTER BEH */
pub const XKB_KEY_Arabic_tehmarbuta             :u32 = 0x05c9;  /* U+0629 ARABIC LETTER TEH MARBUTA */
pub const XKB_KEY_Arabic_teh                    :u32 = 0x05ca;  /* U+062A ARABIC LETTER TEH */
pub const XKB_KEY_Arabic_theh                   :u32 = 0x05cb;  /* U+062B ARABIC LETTER THEH */
pub const XKB_KEY_Arabic_jeem                   :u32 = 0x05cc;  /* U+062C ARABIC LETTER JEEM */
pub const XKB_KEY_Arabic_hah                    :u32 = 0x05cd;  /* U+062D ARABIC LETTER HAH */
pub const XKB_KEY_Arabic_khah                   :u32 = 0x05ce;  /* U+062E ARABIC LETTER KHAH */
pub const XKB_KEY_Arabic_dal                    :u32 = 0x05cf;  /* U+062F ARABIC LETTER DAL */
pub const XKB_KEY_Arabic_thal                   :u32 = 0x05d0;  /* U+0630 ARABIC LETTER THAL */
pub const XKB_KEY_Arabic_ra                     :u32 = 0x05d1;  /* U+0631 ARABIC LETTER REH */
pub const XKB_KEY_Arabic_zain                   :u32 = 0x05d2;  /* U+0632 ARABIC LETTER ZAIN */
pub const XKB_KEY_Arabic_seen                   :u32 = 0x05d3;  /* U+0633 ARABIC LETTER SEEN */
pub const XKB_KEY_Arabic_sheen                  :u32 = 0x05d4;  /* U+0634 ARABIC LETTER SHEEN */
pub const XKB_KEY_Arabic_sad                    :u32 = 0x05d5;  /* U+0635 ARABIC LETTER SAD */
pub const XKB_KEY_Arabic_dad                    :u32 = 0x05d6;  /* U+0636 ARABIC LETTER DAD */
pub const XKB_KEY_Arabic_tah                    :u32 = 0x05d7;  /* U+0637 ARABIC LETTER TAH */
pub const XKB_KEY_Arabic_zah                    :u32 = 0x05d8;  /* U+0638 ARABIC LETTER ZAH */
pub const XKB_KEY_Arabic_ain                    :u32 = 0x05d9;  /* U+0639 ARABIC LETTER AIN */
pub const XKB_KEY_Arabic_ghain                  :u32 = 0x05da;  /* U+063A ARABIC LETTER GHAIN */
pub const XKB_KEY_Arabic_tatweel                :u32 = 0x05e0;  /* U+0640 ARABIC TATWEEL */
pub const XKB_KEY_Arabic_feh                    :u32 = 0x05e1;  /* U+0641 ARABIC LETTER FEH */
pub const XKB_KEY_Arabic_qaf                    :u32 = 0x05e2;  /* U+0642 ARABIC LETTER QAF */
pub const XKB_KEY_Arabic_kaf                    :u32 = 0x05e3;  /* U+0643 ARABIC LETTER KAF */
pub const XKB_KEY_Arabic_lam                    :u32 = 0x05e4;  /* U+0644 ARABIC LETTER LAM */
pub const XKB_KEY_Arabic_meem                   :u32 = 0x05e5;  /* U+0645 ARABIC LETTER MEEM */
pub const XKB_KEY_Arabic_noon                   :u32 = 0x05e6;  /* U+0646 ARABIC LETTER NOON */
pub const XKB_KEY_Arabic_ha                     :u32 = 0x05e7;  /* U+0647 ARABIC LETTER HEH */
pub const XKB_KEY_Arabic_heh                    :u32 = 0x05e7;  /* deprecated */
pub const XKB_KEY_Arabic_waw                    :u32 = 0x05e8;  /* U+0648 ARABIC LETTER WAW */
pub const XKB_KEY_Arabic_alefmaksura            :u32 = 0x05e9;  /* U+0649 ARABIC LETTER ALEF MAKSURA */
pub const XKB_KEY_Arabic_yeh                    :u32 = 0x05ea;  /* U+064A ARABIC LETTER YEH */
pub const XKB_KEY_Arabic_fathatan               :u32 = 0x05eb;  /* U+064B ARABIC FATHATAN */
pub const XKB_KEY_Arabic_dammatan               :u32 = 0x05ec;  /* U+064C ARABIC DAMMATAN */
pub const XKB_KEY_Arabic_kasratan               :u32 = 0x05ed;  /* U+064D ARABIC KASRATAN */
pub const XKB_KEY_Arabic_fatha                  :u32 = 0x05ee;  /* U+064E ARABIC FATHA */
pub const XKB_KEY_Arabic_damma                  :u32 = 0x05ef;  /* U+064F ARABIC DAMMA */
pub const XKB_KEY_Arabic_kasra                  :u32 = 0x05f0;  /* U+0650 ARABIC KASRA */
pub const XKB_KEY_Arabic_shadda                 :u32 = 0x05f1;  /* U+0651 ARABIC SHADDA */
pub const XKB_KEY_Arabic_sukun                  :u32 = 0x05f2;  /* U+0652 ARABIC SUKUN */
pub const XKB_KEY_Arabic_madda_above         :u32 = 0x1000653;  /* U+0653 ARABIC MADDAH ABOVE */
pub const XKB_KEY_Arabic_hamza_above         :u32 = 0x1000654;  /* U+0654 ARABIC HAMZA ABOVE */
pub const XKB_KEY_Arabic_hamza_below         :u32 = 0x1000655;  /* U+0655 ARABIC HAMZA BELOW */
pub const XKB_KEY_Arabic_jeh                 :u32 = 0x1000698;  /* U+0698 ARABIC LETTER JEH */
pub const XKB_KEY_Arabic_veh                 :u32 = 0x10006a4;  /* U+06A4 ARABIC LETTER VEH */
pub const XKB_KEY_Arabic_keheh               :u32 = 0x10006a9;  /* U+06A9 ARABIC LETTER KEHEH */
pub const XKB_KEY_Arabic_gaf                 :u32 = 0x10006af;  /* U+06AF ARABIC LETTER GAF */
pub const XKB_KEY_Arabic_noon_ghunna         :u32 = 0x10006ba;  /* U+06BA ARABIC LETTER NOON GHUNNA */
pub const XKB_KEY_Arabic_heh_doachashmee     :u32 = 0x10006be;  /* U+06BE ARABIC LETTER HEH DOACHASHMEE */
pub const XKB_KEY_Farsi_yeh                  :u32 = 0x10006cc;  /* U+06CC ARABIC LETTER FARSI YEH */
pub const XKB_KEY_Arabic_farsi_yeh           :u32 = 0x10006cc;  /* U+06CC ARABIC LETTER FARSI YEH */
pub const XKB_KEY_Arabic_yeh_baree           :u32 = 0x10006d2;  /* U+06D2 ARABIC LETTER YEH BARREE */
pub const XKB_KEY_Arabic_heh_goal            :u32 = 0x10006c1;  /* U+06C1 ARABIC LETTER HEH GOAL */
pub const XKB_KEY_Arabic_switch                 :u32 = 0xff7e;  /* Alias for mode_switch */

/*
 * Cyrillic
 * Byte 3 = 6
 */
pub const XKB_KEY_Cyrillic_GHE_bar           :u32 = 0x1000492;  /* U+0492 CYRILLIC CAPITAL LETTER GHE WITH STROKE */
pub const XKB_KEY_Cyrillic_ghe_bar           :u32 = 0x1000493;  /* U+0493 CYRILLIC SMALL LETTER GHE WITH STROKE */
pub const XKB_KEY_Cyrillic_ZHE_descender     :u32 = 0x1000496;  /* U+0496 CYRILLIC CAPITAL LETTER ZHE WITH DESCENDER */
pub const XKB_KEY_Cyrillic_zhe_descender     :u32 = 0x1000497;  /* U+0497 CYRILLIC SMALL LETTER ZHE WITH DESCENDER */
pub const XKB_KEY_Cyrillic_KA_descender      :u32 = 0x100049a;  /* U+049A CYRILLIC CAPITAL LETTER KA WITH DESCENDER */
pub const XKB_KEY_Cyrillic_ka_descender      :u32 = 0x100049b;  /* U+049B CYRILLIC SMALL LETTER KA WITH DESCENDER */
pub const XKB_KEY_Cyrillic_KA_vertstroke     :u32 = 0x100049c;  /* U+049C CYRILLIC CAPITAL LETTER KA WITH VERTICAL STROKE */
pub const XKB_KEY_Cyrillic_ka_vertstroke     :u32 = 0x100049d;  /* U+049D CYRILLIC SMALL LETTER KA WITH VERTICAL STROKE */
pub const XKB_KEY_Cyrillic_EN_descender      :u32 = 0x10004a2;  /* U+04A2 CYRILLIC CAPITAL LETTER EN WITH DESCENDER */
pub const XKB_KEY_Cyrillic_en_descender      :u32 = 0x10004a3;  /* U+04A3 CYRILLIC SMALL LETTER EN WITH DESCENDER */
pub const XKB_KEY_Cyrillic_U_straight        :u32 = 0x10004ae;  /* U+04AE CYRILLIC CAPITAL LETTER STRAIGHT U */
pub const XKB_KEY_Cyrillic_u_straight        :u32 = 0x10004af;  /* U+04AF CYRILLIC SMALL LETTER STRAIGHT U */
pub const XKB_KEY_Cyrillic_U_straight_bar    :u32 = 0x10004b0;  /* U+04B0 CYRILLIC CAPITAL LETTER STRAIGHT U WITH STROKE */
pub const XKB_KEY_Cyrillic_u_straight_bar    :u32 = 0x10004b1;  /* U+04B1 CYRILLIC SMALL LETTER STRAIGHT U WITH STROKE */
pub const XKB_KEY_Cyrillic_HA_descender      :u32 = 0x10004b2;  /* U+04B2 CYRILLIC CAPITAL LETTER HA WITH DESCENDER */
pub const XKB_KEY_Cyrillic_ha_descender      :u32 = 0x10004b3;  /* U+04B3 CYRILLIC SMALL LETTER HA WITH DESCENDER */
pub const XKB_KEY_Cyrillic_CHE_descender     :u32 = 0x10004b6;  /* U+04B6 CYRILLIC CAPITAL LETTER CHE WITH DESCENDER */
pub const XKB_KEY_Cyrillic_che_descender     :u32 = 0x10004b7;  /* U+04B7 CYRILLIC SMALL LETTER CHE WITH DESCENDER */
pub const XKB_KEY_Cyrillic_CHE_vertstroke    :u32 = 0x10004b8;  /* U+04B8 CYRILLIC CAPITAL LETTER CHE WITH VERTICAL STROKE */
pub const XKB_KEY_Cyrillic_che_vertstroke    :u32 = 0x10004b9;  /* U+04B9 CYRILLIC SMALL LETTER CHE WITH VERTICAL STROKE */
pub const XKB_KEY_Cyrillic_SHHA              :u32 = 0x10004ba;  /* U+04BA CYRILLIC CAPITAL LETTER SHHA */
pub const XKB_KEY_Cyrillic_shha              :u32 = 0x10004bb;  /* U+04BB CYRILLIC SMALL LETTER SHHA */

pub const XKB_KEY_Cyrillic_SCHWA             :u32 = 0x10004d8;  /* U+04D8 CYRILLIC CAPITAL LETTER SCHWA */
pub const XKB_KEY_Cyrillic_schwa             :u32 = 0x10004d9;  /* U+04D9 CYRILLIC SMALL LETTER SCHWA */
pub const XKB_KEY_Cyrillic_I_macron          :u32 = 0x10004e2;  /* U+04E2 CYRILLIC CAPITAL LETTER I WITH MACRON */
pub const XKB_KEY_Cyrillic_i_macron          :u32 = 0x10004e3;  /* U+04E3 CYRILLIC SMALL LETTER I WITH MACRON */
pub const XKB_KEY_Cyrillic_O_bar             :u32 = 0x10004e8;  /* U+04E8 CYRILLIC CAPITAL LETTER BARRED O */
pub const XKB_KEY_Cyrillic_o_bar             :u32 = 0x10004e9;  /* U+04E9 CYRILLIC SMALL LETTER BARRED O */
pub const XKB_KEY_Cyrillic_U_macron          :u32 = 0x10004ee;  /* U+04EE CYRILLIC CAPITAL LETTER U WITH MACRON */
pub const XKB_KEY_Cyrillic_u_macron          :u32 = 0x10004ef;  /* U+04EF CYRILLIC SMALL LETTER U WITH MACRON */

pub const XKB_KEY_Serbian_dje                   :u32 = 0x06a1;  /* U+0452 CYRILLIC SMALL LETTER DJE */
pub const XKB_KEY_Macedonia_gje                 :u32 = 0x06a2;  /* U+0453 CYRILLIC SMALL LETTER GJE */
pub const XKB_KEY_Cyrillic_io                   :u32 = 0x06a3;  /* U+0451 CYRILLIC SMALL LETTER IO */
pub const XKB_KEY_Ukrainian_ie                  :u32 = 0x06a4;  /* U+0454 CYRILLIC SMALL LETTER UKRAINIAN IE */
pub const XKB_KEY_Ukranian_je                   :u32 = 0x06a4;  /* deprecated */
pub const XKB_KEY_Macedonia_dse                 :u32 = 0x06a5;  /* U+0455 CYRILLIC SMALL LETTER DZE */
pub const XKB_KEY_Ukrainian_i                   :u32 = 0x06a6;  /* U+0456 CYRILLIC SMALL LETTER BYELORUSSIAN-UKRAINIAN I */
pub const XKB_KEY_Ukranian_i                    :u32 = 0x06a6;  /* deprecated */
pub const XKB_KEY_Ukrainian_yi                  :u32 = 0x06a7;  /* U+0457 CYRILLIC SMALL LETTER YI */
pub const XKB_KEY_Ukranian_yi                   :u32 = 0x06a7;  /* deprecated */
pub const XKB_KEY_Cyrillic_je                   :u32 = 0x06a8;  /* U+0458 CYRILLIC SMALL LETTER JE */
pub const XKB_KEY_Serbian_je                    :u32 = 0x06a8;  /* deprecated */
pub const XKB_KEY_Cyrillic_lje                  :u32 = 0x06a9;  /* U+0459 CYRILLIC SMALL LETTER LJE */
pub const XKB_KEY_Serbian_lje                   :u32 = 0x06a9;  /* deprecated */
pub const XKB_KEY_Cyrillic_nje                  :u32 = 0x06aa;  /* U+045A CYRILLIC SMALL LETTER NJE */
pub const XKB_KEY_Serbian_nje                   :u32 = 0x06aa;  /* deprecated */
pub const XKB_KEY_Serbian_tshe                  :u32 = 0x06ab;  /* U+045B CYRILLIC SMALL LETTER TSHE */
pub const XKB_KEY_Macedonia_kje                 :u32 = 0x06ac;  /* U+045C CYRILLIC SMALL LETTER KJE */
pub const XKB_KEY_Ukrainian_ghe_with_upturn     :u32 = 0x06ad;  /* U+0491 CYRILLIC SMALL LETTER GHE WITH UPTURN */
pub const XKB_KEY_Byelorussian_shortu           :u32 = 0x06ae;  /* U+045E CYRILLIC SMALL LETTER SHORT U */
pub const XKB_KEY_Cyrillic_dzhe                 :u32 = 0x06af;  /* U+045F CYRILLIC SMALL LETTER DZHE */
pub const XKB_KEY_Serbian_dze                   :u32 = 0x06af;  /* deprecated */
pub const XKB_KEY_numerosign                    :u32 = 0x06b0;  /* U+2116 NUMERO SIGN */
pub const XKB_KEY_Serbian_DJE                   :u32 = 0x06b1;  /* U+0402 CYRILLIC CAPITAL LETTER DJE */
pub const XKB_KEY_Macedonia_GJE                 :u32 = 0x06b2;  /* U+0403 CYRILLIC CAPITAL LETTER GJE */
pub const XKB_KEY_Cyrillic_IO                   :u32 = 0x06b3;  /* U+0401 CYRILLIC CAPITAL LETTER IO */
pub const XKB_KEY_Ukrainian_IE                  :u32 = 0x06b4;  /* U+0404 CYRILLIC CAPITAL LETTER UKRAINIAN IE */
pub const XKB_KEY_Ukranian_JE                   :u32 = 0x06b4;  /* deprecated */
pub const XKB_KEY_Macedonia_DSE                 :u32 = 0x06b5;  /* U+0405 CYRILLIC CAPITAL LETTER DZE */
pub const XKB_KEY_Ukrainian_I                   :u32 = 0x06b6;  /* U+0406 CYRILLIC CAPITAL LETTER BYELORUSSIAN-UKRAINIAN I */
pub const XKB_KEY_Ukranian_I                    :u32 = 0x06b6;  /* deprecated */
pub const XKB_KEY_Ukrainian_YI                  :u32 = 0x06b7;  /* U+0407 CYRILLIC CAPITAL LETTER YI */
pub const XKB_KEY_Ukranian_YI                   :u32 = 0x06b7;  /* deprecated */
pub const XKB_KEY_Cyrillic_JE                   :u32 = 0x06b8;  /* U+0408 CYRILLIC CAPITAL LETTER JE */
pub const XKB_KEY_Serbian_JE                    :u32 = 0x06b8;  /* deprecated */
pub const XKB_KEY_Cyrillic_LJE                  :u32 = 0x06b9;  /* U+0409 CYRILLIC CAPITAL LETTER LJE */
pub const XKB_KEY_Serbian_LJE                   :u32 = 0x06b9;  /* deprecated */
pub const XKB_KEY_Cyrillic_NJE                  :u32 = 0x06ba;  /* U+040A CYRILLIC CAPITAL LETTER NJE */
pub const XKB_KEY_Serbian_NJE                   :u32 = 0x06ba;  /* deprecated */
pub const XKB_KEY_Serbian_TSHE                  :u32 = 0x06bb;  /* U+040B CYRILLIC CAPITAL LETTER TSHE */
pub const XKB_KEY_Macedonia_KJE                 :u32 = 0x06bc;  /* U+040C CYRILLIC CAPITAL LETTER KJE */
pub const XKB_KEY_Ukrainian_GHE_WITH_UPTURN     :u32 = 0x06bd;  /* U+0490 CYRILLIC CAPITAL LETTER GHE WITH UPTURN */
pub const XKB_KEY_Byelorussian_SHORTU           :u32 = 0x06be;  /* U+040E CYRILLIC CAPITAL LETTER SHORT U */
pub const XKB_KEY_Cyrillic_DZHE                 :u32 = 0x06bf;  /* U+040F CYRILLIC CAPITAL LETTER DZHE */
pub const XKB_KEY_Serbian_DZE                   :u32 = 0x06bf;  /* deprecated */
pub const XKB_KEY_Cyrillic_yu                   :u32 = 0x06c0;  /* U+044E CYRILLIC SMALL LETTER YU */
pub const XKB_KEY_Cyrillic_a                    :u32 = 0x06c1;  /* U+0430 CYRILLIC SMALL LETTER A */
pub const XKB_KEY_Cyrillic_be                   :u32 = 0x06c2;  /* U+0431 CYRILLIC SMALL LETTER BE */
pub const XKB_KEY_Cyrillic_tse                  :u32 = 0x06c3;  /* U+0446 CYRILLIC SMALL LETTER TSE */
pub const XKB_KEY_Cyrillic_de                   :u32 = 0x06c4;  /* U+0434 CYRILLIC SMALL LETTER DE */
pub const XKB_KEY_Cyrillic_ie                   :u32 = 0x06c5;  /* U+0435 CYRILLIC SMALL LETTER IE */
pub const XKB_KEY_Cyrillic_ef                   :u32 = 0x06c6;  /* U+0444 CYRILLIC SMALL LETTER EF */
pub const XKB_KEY_Cyrillic_ghe                  :u32 = 0x06c7;  /* U+0433 CYRILLIC SMALL LETTER GHE */
pub const XKB_KEY_Cyrillic_ha                   :u32 = 0x06c8;  /* U+0445 CYRILLIC SMALL LETTER HA */
pub const XKB_KEY_Cyrillic_i                    :u32 = 0x06c9;  /* U+0438 CYRILLIC SMALL LETTER I */
pub const XKB_KEY_Cyrillic_shorti               :u32 = 0x06ca;  /* U+0439 CYRILLIC SMALL LETTER SHORT I */
pub const XKB_KEY_Cyrillic_ka                   :u32 = 0x06cb;  /* U+043A CYRILLIC SMALL LETTER KA */
pub const XKB_KEY_Cyrillic_el                   :u32 = 0x06cc;  /* U+043B CYRILLIC SMALL LETTER EL */
pub const XKB_KEY_Cyrillic_em                   :u32 = 0x06cd;  /* U+043C CYRILLIC SMALL LETTER EM */
pub const XKB_KEY_Cyrillic_en                   :u32 = 0x06ce;  /* U+043D CYRILLIC SMALL LETTER EN */
pub const XKB_KEY_Cyrillic_o                    :u32 = 0x06cf;  /* U+043E CYRILLIC SMALL LETTER O */
pub const XKB_KEY_Cyrillic_pe                   :u32 = 0x06d0;  /* U+043F CYRILLIC SMALL LETTER PE */
pub const XKB_KEY_Cyrillic_ya                   :u32 = 0x06d1;  /* U+044F CYRILLIC SMALL LETTER YA */
pub const XKB_KEY_Cyrillic_er                   :u32 = 0x06d2;  /* U+0440 CYRILLIC SMALL LETTER ER */
pub const XKB_KEY_Cyrillic_es                   :u32 = 0x06d3;  /* U+0441 CYRILLIC SMALL LETTER ES */
pub const XKB_KEY_Cyrillic_te                   :u32 = 0x06d4;  /* U+0442 CYRILLIC SMALL LETTER TE */
pub const XKB_KEY_Cyrillic_u                    :u32 = 0x06d5;  /* U+0443 CYRILLIC SMALL LETTER U */
pub const XKB_KEY_Cyrillic_zhe                  :u32 = 0x06d6;  /* U+0436 CYRILLIC SMALL LETTER ZHE */
pub const XKB_KEY_Cyrillic_ve                   :u32 = 0x06d7;  /* U+0432 CYRILLIC SMALL LETTER VE */
pub const XKB_KEY_Cyrillic_softsign             :u32 = 0x06d8;  /* U+044C CYRILLIC SMALL LETTER SOFT SIGN */
pub const XKB_KEY_Cyrillic_yeru                 :u32 = 0x06d9;  /* U+044B CYRILLIC SMALL LETTER YERU */
pub const XKB_KEY_Cyrillic_ze                   :u32 = 0x06da;  /* U+0437 CYRILLIC SMALL LETTER ZE */
pub const XKB_KEY_Cyrillic_sha                  :u32 = 0x06db;  /* U+0448 CYRILLIC SMALL LETTER SHA */
pub const XKB_KEY_Cyrillic_e                    :u32 = 0x06dc;  /* U+044D CYRILLIC SMALL LETTER E */
pub const XKB_KEY_Cyrillic_shcha                :u32 = 0x06dd;  /* U+0449 CYRILLIC SMALL LETTER SHCHA */
pub const XKB_KEY_Cyrillic_che                  :u32 = 0x06de;  /* U+0447 CYRILLIC SMALL LETTER CHE */
pub const XKB_KEY_Cyrillic_hardsign             :u32 = 0x06df;  /* U+044A CYRILLIC SMALL LETTER HARD SIGN */
pub const XKB_KEY_Cyrillic_YU                   :u32 = 0x06e0;  /* U+042E CYRILLIC CAPITAL LETTER YU */
pub const XKB_KEY_Cyrillic_A                    :u32 = 0x06e1;  /* U+0410 CYRILLIC CAPITAL LETTER A */
pub const XKB_KEY_Cyrillic_BE                   :u32 = 0x06e2;  /* U+0411 CYRILLIC CAPITAL LETTER BE */
pub const XKB_KEY_Cyrillic_TSE                  :u32 = 0x06e3;  /* U+0426 CYRILLIC CAPITAL LETTER TSE */
pub const XKB_KEY_Cyrillic_DE                   :u32 = 0x06e4;  /* U+0414 CYRILLIC CAPITAL LETTER DE */
pub const XKB_KEY_Cyrillic_IE                   :u32 = 0x06e5;  /* U+0415 CYRILLIC CAPITAL LETTER IE */
pub const XKB_KEY_Cyrillic_EF                   :u32 = 0x06e6;  /* U+0424 CYRILLIC CAPITAL LETTER EF */
pub const XKB_KEY_Cyrillic_GHE                  :u32 = 0x06e7;  /* U+0413 CYRILLIC CAPITAL LETTER GHE */
pub const XKB_KEY_Cyrillic_HA                   :u32 = 0x06e8;  /* U+0425 CYRILLIC CAPITAL LETTER HA */
pub const XKB_KEY_Cyrillic_I                    :u32 = 0x06e9;  /* U+0418 CYRILLIC CAPITAL LETTER I */
pub const XKB_KEY_Cyrillic_SHORTI               :u32 = 0x06ea;  /* U+0419 CYRILLIC CAPITAL LETTER SHORT I */
pub const XKB_KEY_Cyrillic_KA                   :u32 = 0x06eb;  /* U+041A CYRILLIC CAPITAL LETTER KA */
pub const XKB_KEY_Cyrillic_EL                   :u32 = 0x06ec;  /* U+041B CYRILLIC CAPITAL LETTER EL */
pub const XKB_KEY_Cyrillic_EM                   :u32 = 0x06ed;  /* U+041C CYRILLIC CAPITAL LETTER EM */
pub const XKB_KEY_Cyrillic_EN                   :u32 = 0x06ee;  /* U+041D CYRILLIC CAPITAL LETTER EN */
pub const XKB_KEY_Cyrillic_O                    :u32 = 0x06ef;  /* U+041E CYRILLIC CAPITAL LETTER O */
pub const XKB_KEY_Cyrillic_PE                   :u32 = 0x06f0;  /* U+041F CYRILLIC CAPITAL LETTER PE */
pub const XKB_KEY_Cyrillic_YA                   :u32 = 0x06f1;  /* U+042F CYRILLIC CAPITAL LETTER YA */
pub const XKB_KEY_Cyrillic_ER                   :u32 = 0x06f2;  /* U+0420 CYRILLIC CAPITAL LETTER ER */
pub const XKB_KEY_Cyrillic_ES                   :u32 = 0x06f3;  /* U+0421 CYRILLIC CAPITAL LETTER ES */
pub const XKB_KEY_Cyrillic_TE                   :u32 = 0x06f4;  /* U+0422 CYRILLIC CAPITAL LETTER TE */
pub const XKB_KEY_Cyrillic_U                    :u32 = 0x06f5;  /* U+0423 CYRILLIC CAPITAL LETTER U */
pub const XKB_KEY_Cyrillic_ZHE                  :u32 = 0x06f6;  /* U+0416 CYRILLIC CAPITAL LETTER ZHE */
pub const XKB_KEY_Cyrillic_VE                   :u32 = 0x06f7;  /* U+0412 CYRILLIC CAPITAL LETTER VE */
pub const XKB_KEY_Cyrillic_SOFTSIGN             :u32 = 0x06f8;  /* U+042C CYRILLIC CAPITAL LETTER SOFT SIGN */
pub const XKB_KEY_Cyrillic_YERU                 :u32 = 0x06f9;  /* U+042B CYRILLIC CAPITAL LETTER YERU */
pub const XKB_KEY_Cyrillic_ZE                   :u32 = 0x06fa;  /* U+0417 CYRILLIC CAPITAL LETTER ZE */
pub const XKB_KEY_Cyrillic_SHA                  :u32 = 0x06fb;  /* U+0428 CYRILLIC CAPITAL LETTER SHA */
pub const XKB_KEY_Cyrillic_E                    :u32 = 0x06fc;  /* U+042D CYRILLIC CAPITAL LETTER E */
pub const XKB_KEY_Cyrillic_SHCHA                :u32 = 0x06fd;  /* U+0429 CYRILLIC CAPITAL LETTER SHCHA */
pub const XKB_KEY_Cyrillic_CHE                  :u32 = 0x06fe;  /* U+0427 CYRILLIC CAPITAL LETTER CHE */
pub const XKB_KEY_Cyrillic_HARDSIGN             :u32 = 0x06ff;  /* U+042A CYRILLIC CAPITAL LETTER HARD SIGN */

/*
 * Greek
 * (based on an early draft of, and not quite identical to, ISO/IEC 8859-7)
 * Byte 3 = 7
 */

pub const XKB_KEY_Greek_ALPHAaccent             :u32 = 0x07a1;  /* U+0386 GREEK CAPITAL LETTER ALPHA WITH TONOS */
pub const XKB_KEY_Greek_EPSILONaccent           :u32 = 0x07a2;  /* U+0388 GREEK CAPITAL LETTER EPSILON WITH TONOS */
pub const XKB_KEY_Greek_ETAaccent               :u32 = 0x07a3;  /* U+0389 GREEK CAPITAL LETTER ETA WITH TONOS */
pub const XKB_KEY_Greek_IOTAaccent              :u32 = 0x07a4;  /* U+038A GREEK CAPITAL LETTER IOTA WITH TONOS */
pub const XKB_KEY_Greek_IOTAdieresis            :u32 = 0x07a5;  /* U+03AA GREEK CAPITAL LETTER IOTA WITH DIALYTIKA */
pub const XKB_KEY_Greek_IOTAdiaeresis           :u32 = 0x07a5;  /* old typo */
pub const XKB_KEY_Greek_OMICRONaccent           :u32 = 0x07a7;  /* U+038C GREEK CAPITAL LETTER OMICRON WITH TONOS */
pub const XKB_KEY_Greek_UPSILONaccent           :u32 = 0x07a8;  /* U+038E GREEK CAPITAL LETTER UPSILON WITH TONOS */
pub const XKB_KEY_Greek_UPSILONdieresis         :u32 = 0x07a9;  /* U+03AB GREEK CAPITAL LETTER UPSILON WITH DIALYTIKA */
pub const XKB_KEY_Greek_OMEGAaccent             :u32 = 0x07ab;  /* U+038F GREEK CAPITAL LETTER OMEGA WITH TONOS */
pub const XKB_KEY_Greek_accentdieresis          :u32 = 0x07ae;  /* U+0385 GREEK DIALYTIKA TONOS */
pub const XKB_KEY_Greek_horizbar                :u32 = 0x07af;  /* U+2015 HORIZONTAL BAR */
pub const XKB_KEY_Greek_alphaaccent             :u32 = 0x07b1;  /* U+03AC GREEK SMALL LETTER ALPHA WITH TONOS */
pub const XKB_KEY_Greek_epsilonaccent           :u32 = 0x07b2;  /* U+03AD GREEK SMALL LETTER EPSILON WITH TONOS */
pub const XKB_KEY_Greek_etaaccent               :u32 = 0x07b3;  /* U+03AE GREEK SMALL LETTER ETA WITH TONOS */
pub const XKB_KEY_Greek_iotaaccent              :u32 = 0x07b4;  /* U+03AF GREEK SMALL LETTER IOTA WITH TONOS */
pub const XKB_KEY_Greek_iotadieresis            :u32 = 0x07b5;  /* U+03CA GREEK SMALL LETTER IOTA WITH DIALYTIKA */
pub const XKB_KEY_Greek_iotaaccentdieresis      :u32 = 0x07b6;  /* U+0390 GREEK SMALL LETTER IOTA WITH DIALYTIKA AND TONOS */
pub const XKB_KEY_Greek_omicronaccent           :u32 = 0x07b7;  /* U+03CC GREEK SMALL LETTER OMICRON WITH TONOS */
pub const XKB_KEY_Greek_upsilonaccent           :u32 = 0x07b8;  /* U+03CD GREEK SMALL LETTER UPSILON WITH TONOS */
pub const XKB_KEY_Greek_upsilondieresis         :u32 = 0x07b9;  /* U+03CB GREEK SMALL LETTER UPSILON WITH DIALYTIKA */
pub const XKB_KEY_Greek_upsilonaccentdieresis   :u32 = 0x07ba;  /* U+03B0 GREEK SMALL LETTER UPSILON WITH DIALYTIKA AND TONOS */
pub const XKB_KEY_Greek_omegaaccent             :u32 = 0x07bb;  /* U+03CE GREEK SMALL LETTER OMEGA WITH TONOS */
pub const XKB_KEY_Greek_ALPHA                   :u32 = 0x07c1;  /* U+0391 GREEK CAPITAL LETTER ALPHA */
pub const XKB_KEY_Greek_BETA                    :u32 = 0x07c2;  /* U+0392 GREEK CAPITAL LETTER BETA */
pub const XKB_KEY_Greek_GAMMA                   :u32 = 0x07c3;  /* U+0393 GREEK CAPITAL LETTER GAMMA */
pub const XKB_KEY_Greek_DELTA                   :u32 = 0x07c4;  /* U+0394 GREEK CAPITAL LETTER DELTA */
pub const XKB_KEY_Greek_EPSILON                 :u32 = 0x07c5;  /* U+0395 GREEK CAPITAL LETTER EPSILON */
pub const XKB_KEY_Greek_ZETA                    :u32 = 0x07c6;  /* U+0396 GREEK CAPITAL LETTER ZETA */
pub const XKB_KEY_Greek_ETA                     :u32 = 0x07c7;  /* U+0397 GREEK CAPITAL LETTER ETA */
pub const XKB_KEY_Greek_THETA                   :u32 = 0x07c8;  /* U+0398 GREEK CAPITAL LETTER THETA */
pub const XKB_KEY_Greek_IOTA                    :u32 = 0x07c9;  /* U+0399 GREEK CAPITAL LETTER IOTA */
pub const XKB_KEY_Greek_KAPPA                   :u32 = 0x07ca;  /* U+039A GREEK CAPITAL LETTER KAPPA */
pub const XKB_KEY_Greek_LAMDA                   :u32 = 0x07cb;  /* U+039B GREEK CAPITAL LETTER LAMDA */
pub const XKB_KEY_Greek_LAMBDA                  :u32 = 0x07cb;  /* U+039B GREEK CAPITAL LETTER LAMDA */
pub const XKB_KEY_Greek_MU                      :u32 = 0x07cc;  /* U+039C GREEK CAPITAL LETTER MU */
pub const XKB_KEY_Greek_NU                      :u32 = 0x07cd;  /* U+039D GREEK CAPITAL LETTER NU */
pub const XKB_KEY_Greek_XI                      :u32 = 0x07ce;  /* U+039E GREEK CAPITAL LETTER XI */
pub const XKB_KEY_Greek_OMICRON                 :u32 = 0x07cf;  /* U+039F GREEK CAPITAL LETTER OMICRON */
pub const XKB_KEY_Greek_PI                      :u32 = 0x07d0;  /* U+03A0 GREEK CAPITAL LETTER PI */
pub const XKB_KEY_Greek_RHO                     :u32 = 0x07d1;  /* U+03A1 GREEK CAPITAL LETTER RHO */
pub const XKB_KEY_Greek_SIGMA                   :u32 = 0x07d2;  /* U+03A3 GREEK CAPITAL LETTER SIGMA */
pub const XKB_KEY_Greek_TAU                     :u32 = 0x07d4;  /* U+03A4 GREEK CAPITAL LETTER TAU */
pub const XKB_KEY_Greek_UPSILON                 :u32 = 0x07d5;  /* U+03A5 GREEK CAPITAL LETTER UPSILON */
pub const XKB_KEY_Greek_PHI                     :u32 = 0x07d6;  /* U+03A6 GREEK CAPITAL LETTER PHI */
pub const XKB_KEY_Greek_CHI                     :u32 = 0x07d7;  /* U+03A7 GREEK CAPITAL LETTER CHI */
pub const XKB_KEY_Greek_PSI                     :u32 = 0x07d8;  /* U+03A8 GREEK CAPITAL LETTER PSI */
pub const XKB_KEY_Greek_OMEGA                   :u32 = 0x07d9;  /* U+03A9 GREEK CAPITAL LETTER OMEGA */
pub const XKB_KEY_Greek_alpha                   :u32 = 0x07e1;  /* U+03B1 GREEK SMALL LETTER ALPHA */
pub const XKB_KEY_Greek_beta                    :u32 = 0x07e2;  /* U+03B2 GREEK SMALL LETTER BETA */
pub const XKB_KEY_Greek_gamma                   :u32 = 0x07e3;  /* U+03B3 GREEK SMALL LETTER GAMMA */
pub const XKB_KEY_Greek_delta                   :u32 = 0x07e4;  /* U+03B4 GREEK SMALL LETTER DELTA */
pub const XKB_KEY_Greek_epsilon                 :u32 = 0x07e5;  /* U+03B5 GREEK SMALL LETTER EPSILON */
pub const XKB_KEY_Greek_zeta                    :u32 = 0x07e6;  /* U+03B6 GREEK SMALL LETTER ZETA */
pub const XKB_KEY_Greek_eta                     :u32 = 0x07e7;  /* U+03B7 GREEK SMALL LETTER ETA */
pub const XKB_KEY_Greek_theta                   :u32 = 0x07e8;  /* U+03B8 GREEK SMALL LETTER THETA */
pub const XKB_KEY_Greek_iota                    :u32 = 0x07e9;  /* U+03B9 GREEK SMALL LETTER IOTA */
pub const XKB_KEY_Greek_kappa                   :u32 = 0x07ea;  /* U+03BA GREEK SMALL LETTER KAPPA */
pub const XKB_KEY_Greek_lamda                   :u32 = 0x07eb;  /* U+03BB GREEK SMALL LETTER LAMDA */
pub const XKB_KEY_Greek_lambda                  :u32 = 0x07eb;  /* U+03BB GREEK SMALL LETTER LAMDA */
pub const XKB_KEY_Greek_mu                      :u32 = 0x07ec;  /* U+03BC GREEK SMALL LETTER MU */
pub const XKB_KEY_Greek_nu                      :u32 = 0x07ed;  /* U+03BD GREEK SMALL LETTER NU */
pub const XKB_KEY_Greek_xi                      :u32 = 0x07ee;  /* U+03BE GREEK SMALL LETTER XI */
pub const XKB_KEY_Greek_omicron                 :u32 = 0x07ef;  /* U+03BF GREEK SMALL LETTER OMICRON */
pub const XKB_KEY_Greek_pi                      :u32 = 0x07f0;  /* U+03C0 GREEK SMALL LETTER PI */
pub const XKB_KEY_Greek_rho                     :u32 = 0x07f1;  /* U+03C1 GREEK SMALL LETTER RHO */
pub const XKB_KEY_Greek_sigma                   :u32 = 0x07f2;  /* U+03C3 GREEK SMALL LETTER SIGMA */
pub const XKB_KEY_Greek_finalsmallsigma         :u32 = 0x07f3;  /* U+03C2 GREEK SMALL LETTER FINAL SIGMA */
pub const XKB_KEY_Greek_tau                     :u32 = 0x07f4;  /* U+03C4 GREEK SMALL LETTER TAU */
pub const XKB_KEY_Greek_upsilon                 :u32 = 0x07f5;  /* U+03C5 GREEK SMALL LETTER UPSILON */
pub const XKB_KEY_Greek_phi                     :u32 = 0x07f6;  /* U+03C6 GREEK SMALL LETTER PHI */
pub const XKB_KEY_Greek_chi                     :u32 = 0x07f7;  /* U+03C7 GREEK SMALL LETTER CHI */
pub const XKB_KEY_Greek_psi                     :u32 = 0x07f8;  /* U+03C8 GREEK SMALL LETTER PSI */
pub const XKB_KEY_Greek_omega                   :u32 = 0x07f9;  /* U+03C9 GREEK SMALL LETTER OMEGA */
pub const XKB_KEY_Greek_switch                  :u32 = 0xff7e;  /* Alias for mode_switch */

/*
 * Technical
 * (from the DEC VT330/VT420 Technical Character Set, http://vt100.net/charsets/technical.html)
 * Byte 3 = 8
 */

pub const XKB_KEY_leftradical                   :u32 = 0x08a1;  /* U+23B7 RADICAL SYMBOL BOTTOM */
pub const XKB_KEY_topleftradical                :u32 = 0x08a2;  /*(U+250C BOX DRAWINGS LIGHT DOWN AND RIGHT)*/
pub const XKB_KEY_horizconnector                :u32 = 0x08a3;  /*(U+2500 BOX DRAWINGS LIGHT HORIZONTAL)*/
pub const XKB_KEY_topintegral                   :u32 = 0x08a4;  /* U+2320 TOP HALF INTEGRAL */
pub const XKB_KEY_botintegral                   :u32 = 0x08a5;  /* U+2321 BOTTOM HALF INTEGRAL */
pub const XKB_KEY_vertconnector                 :u32 = 0x08a6;  /*(U+2502 BOX DRAWINGS LIGHT VERTICAL)*/
pub const XKB_KEY_topleftsqbracket              :u32 = 0x08a7;  /* U+23A1 LEFT SQUARE BRACKET UPPER CORNER */
pub const XKB_KEY_botleftsqbracket              :u32 = 0x08a8;  /* U+23A3 LEFT SQUARE BRACKET LOWER CORNER */
pub const XKB_KEY_toprightsqbracket             :u32 = 0x08a9;  /* U+23A4 RIGHT SQUARE BRACKET UPPER CORNER */
pub const XKB_KEY_botrightsqbracket             :u32 = 0x08aa;  /* U+23A6 RIGHT SQUARE BRACKET LOWER CORNER */
pub const XKB_KEY_topleftparens                 :u32 = 0x08ab;  /* U+239B LEFT PARENTHESIS UPPER HOOK */
pub const XKB_KEY_botleftparens                 :u32 = 0x08ac;  /* U+239D LEFT PARENTHESIS LOWER HOOK */
pub const XKB_KEY_toprightparens                :u32 = 0x08ad;  /* U+239E RIGHT PARENTHESIS UPPER HOOK */
pub const XKB_KEY_botrightparens                :u32 = 0x08ae;  /* U+23A0 RIGHT PARENTHESIS LOWER HOOK */
pub const XKB_KEY_leftmiddlecurlybrace          :u32 = 0x08af;  /* U+23A8 LEFT CURLY BRACKET MIDDLE PIECE */
pub const XKB_KEY_rightmiddlecurlybrace         :u32 = 0x08b0;  /* U+23AC RIGHT CURLY BRACKET MIDDLE PIECE */
pub const XKB_KEY_topleftsummation              :u32 = 0x08b1;
pub const XKB_KEY_botleftsummation              :u32 = 0x08b2;
pub const XKB_KEY_topvertsummationconnector     :u32 = 0x08b3;
pub const XKB_KEY_botvertsummationconnector     :u32 = 0x08b4;
pub const XKB_KEY_toprightsummation             :u32 = 0x08b5;
pub const XKB_KEY_botrightsummation             :u32 = 0x08b6;
pub const XKB_KEY_rightmiddlesummation          :u32 = 0x08b7;
pub const XKB_KEY_lessthanequal                 :u32 = 0x08bc;  /* U+2264 LESS-THAN OR EQUAL TO */
pub const XKB_KEY_notequal                      :u32 = 0x08bd;  /* U+2260 NOT EQUAL TO */
pub const XKB_KEY_greaterthanequal              :u32 = 0x08be;  /* U+2265 GREATER-THAN OR EQUAL TO */
pub const XKB_KEY_integral                      :u32 = 0x08bf;  /* U+222B INTEGRAL */
pub const XKB_KEY_therefore                     :u32 = 0x08c0;  /* U+2234 THEREFORE */
pub const XKB_KEY_variation                     :u32 = 0x08c1;  /* U+221D PROPORTIONAL TO */
pub const XKB_KEY_infinity                      :u32 = 0x08c2;  /* U+221E INFINITY */
pub const XKB_KEY_nabla                         :u32 = 0x08c5;  /* U+2207 NABLA */
pub const XKB_KEY_approximate                   :u32 = 0x08c8;  /* U+223C TILDE OPERATOR */
pub const XKB_KEY_similarequal                  :u32 = 0x08c9;  /* U+2243 ASYMPTOTICALLY EQUAL TO */
pub const XKB_KEY_ifonlyif                      :u32 = 0x08cd;  /* U+21D4 LEFT RIGHT DOUBLE ARROW */
pub const XKB_KEY_implies                       :u32 = 0x08ce;  /* U+21D2 RIGHTWARDS DOUBLE ARROW */
pub const XKB_KEY_identical                     :u32 = 0x08cf;  /* U+2261 IDENTICAL TO */
pub const XKB_KEY_radical                       :u32 = 0x08d6;  /* U+221A SQUARE ROOT */
pub const XKB_KEY_includedin                    :u32 = 0x08da;  /* U+2282 SUBSET OF */
pub const XKB_KEY_includes                      :u32 = 0x08db;  /* U+2283 SUPERSET OF */
pub const XKB_KEY_intersection                  :u32 = 0x08dc;  /* U+2229 INTERSECTION */
pub const XKB_KEY_union                         :u32 = 0x08dd;  /* U+222A UNION */
pub const XKB_KEY_logicaland                    :u32 = 0x08de;  /* U+2227 LOGICAL AND */
pub const XKB_KEY_logicalor                     :u32 = 0x08df;  /* U+2228 LOGICAL OR */
pub const XKB_KEY_partialderivative             :u32 = 0x08ef;  /* U+2202 PARTIAL DIFFERENTIAL */
pub const XKB_KEY_function                      :u32 = 0x08f6;  /* U+0192 LATIN SMALL LETTER F WITH HOOK */
pub const XKB_KEY_leftarrow                     :u32 = 0x08fb;  /* U+2190 LEFTWARDS ARROW */
pub const XKB_KEY_uparrow                       :u32 = 0x08fc;  /* U+2191 UPWARDS ARROW */
pub const XKB_KEY_rightarrow                    :u32 = 0x08fd;  /* U+2192 RIGHTWARDS ARROW */
pub const XKB_KEY_downarrow                     :u32 = 0x08fe;  /* U+2193 DOWNWARDS ARROW */

/*
 * Special
 * (from the DEC VT100 Special Graphics Character Set)
 * Byte 3 = 9
 */

pub const XKB_KEY_blank                         :u32 = 0x09df;
pub const XKB_KEY_soliddiamond                  :u32 = 0x09e0;  /* U+25C6 BLACK DIAMOND */
pub const XKB_KEY_checkerboard                  :u32 = 0x09e1;  /* U+2592 MEDIUM SHADE */
pub const XKB_KEY_ht                            :u32 = 0x09e2;  /* U+2409 SYMBOL FOR HORIZONTAL TABULATION */
pub const XKB_KEY_ff                            :u32 = 0x09e3;  /* U+240C SYMBOL FOR FORM FEED */
pub const XKB_KEY_cr                            :u32 = 0x09e4;  /* U+240D SYMBOL FOR CARRIAGE RETURN */
pub const XKB_KEY_lf                            :u32 = 0x09e5;  /* U+240A SYMBOL FOR LINE FEED */
pub const XKB_KEY_nl                            :u32 = 0x09e8;  /* U+2424 SYMBOL FOR NEWLINE */
pub const XKB_KEY_vt                            :u32 = 0x09e9;  /* U+240B SYMBOL FOR VERTICAL TABULATION */
pub const XKB_KEY_lowrightcorner                :u32 = 0x09ea;  /* U+2518 BOX DRAWINGS LIGHT UP AND LEFT */
pub const XKB_KEY_uprightcorner                 :u32 = 0x09eb;  /* U+2510 BOX DRAWINGS LIGHT DOWN AND LEFT */
pub const XKB_KEY_upleftcorner                  :u32 = 0x09ec;  /* U+250C BOX DRAWINGS LIGHT DOWN AND RIGHT */
pub const XKB_KEY_lowleftcorner                 :u32 = 0x09ed;  /* U+2514 BOX DRAWINGS LIGHT UP AND RIGHT */
pub const XKB_KEY_crossinglines                 :u32 = 0x09ee;  /* U+253C BOX DRAWINGS LIGHT VERTICAL AND HORIZONTAL */
pub const XKB_KEY_horizlinescan1                :u32 = 0x09ef;  /* U+23BA HORIZONTAL SCAN LINE-1 */
pub const XKB_KEY_horizlinescan3                :u32 = 0x09f0;  /* U+23BB HORIZONTAL SCAN LINE-3 */
pub const XKB_KEY_horizlinescan5                :u32 = 0x09f1;  /* U+2500 BOX DRAWINGS LIGHT HORIZONTAL */
pub const XKB_KEY_horizlinescan7                :u32 = 0x09f2;  /* U+23BC HORIZONTAL SCAN LINE-7 */
pub const XKB_KEY_horizlinescan9                :u32 = 0x09f3;  /* U+23BD HORIZONTAL SCAN LINE-9 */
pub const XKB_KEY_leftt                         :u32 = 0x09f4;  /* U+251C BOX DRAWINGS LIGHT VERTICAL AND RIGHT */
pub const XKB_KEY_rightt                        :u32 = 0x09f5;  /* U+2524 BOX DRAWINGS LIGHT VERTICAL AND LEFT */
pub const XKB_KEY_bott                          :u32 = 0x09f6;  /* U+2534 BOX DRAWINGS LIGHT UP AND HORIZONTAL */
pub const XKB_KEY_topt                          :u32 = 0x09f7;  /* U+252C BOX DRAWINGS LIGHT DOWN AND HORIZONTAL */
pub const XKB_KEY_vertbar                       :u32 = 0x09f8;  /* U+2502 BOX DRAWINGS LIGHT VERTICAL */

/*
 * Publishing
 * (these are probably from a long forgotten DEC Publishing
 * font that once shipped with DECwrite)
 * Byte 3 = :u32 = 0x0a;
 */

pub const XKB_KEY_emspace                       :u32 = 0x0aa1;  /* U+2003 EM SPACE */
pub const XKB_KEY_enspace                       :u32 = 0x0aa2;  /* U+2002 EN SPACE */
pub const XKB_KEY_em3space                      :u32 = 0x0aa3;  /* U+2004 THREE-PER-EM SPACE */
pub const XKB_KEY_em4space                      :u32 = 0x0aa4;  /* U+2005 FOUR-PER-EM SPACE */
pub const XKB_KEY_digitspace                    :u32 = 0x0aa5;  /* U+2007 FIGURE SPACE */
pub const XKB_KEY_punctspace                    :u32 = 0x0aa6;  /* U+2008 PUNCTUATION SPACE */
pub const XKB_KEY_thinspace                     :u32 = 0x0aa7;  /* U+2009 THIN SPACE */
pub const XKB_KEY_hairspace                     :u32 = 0x0aa8;  /* U+200A HAIR SPACE */
pub const XKB_KEY_emdash                        :u32 = 0x0aa9;  /* U+2014 EM DASH */
pub const XKB_KEY_endash                        :u32 = 0x0aaa;  /* U+2013 EN DASH */
pub const XKB_KEY_signifblank                   :u32 = 0x0aac;  /*(U+2423 OPEN BOX)*/
pub const XKB_KEY_ellipsis                      :u32 = 0x0aae;  /* U+2026 HORIZONTAL ELLIPSIS */
pub const XKB_KEY_doubbaselinedot               :u32 = 0x0aaf;  /* U+2025 TWO DOT LEADER */
pub const XKB_KEY_onethird                      :u32 = 0x0ab0;  /* U+2153 VULGAR FRACTION ONE THIRD */
pub const XKB_KEY_twothirds                     :u32 = 0x0ab1;  /* U+2154 VULGAR FRACTION TWO THIRDS */
pub const XKB_KEY_onefifth                      :u32 = 0x0ab2;  /* U+2155 VULGAR FRACTION ONE FIFTH */
pub const XKB_KEY_twofifths                     :u32 = 0x0ab3;  /* U+2156 VULGAR FRACTION TWO FIFTHS */
pub const XKB_KEY_threefifths                   :u32 = 0x0ab4;  /* U+2157 VULGAR FRACTION THREE FIFTHS */
pub const XKB_KEY_fourfifths                    :u32 = 0x0ab5;  /* U+2158 VULGAR FRACTION FOUR FIFTHS */
pub const XKB_KEY_onesixth                      :u32 = 0x0ab6;  /* U+2159 VULGAR FRACTION ONE SIXTH */
pub const XKB_KEY_fivesixths                    :u32 = 0x0ab7;  /* U+215A VULGAR FRACTION FIVE SIXTHS */
pub const XKB_KEY_careof                        :u32 = 0x0ab8;  /* U+2105 CARE OF */
pub const XKB_KEY_figdash                       :u32 = 0x0abb;  /* U+2012 FIGURE DASH */
pub const XKB_KEY_leftanglebracket              :u32 = 0x0abc;  /*(U+27E8 MATHEMATICAL LEFT ANGLE BRACKET)*/
pub const XKB_KEY_decimalpoint                  :u32 = 0x0abd;  /*(U+002E FULL STOP)*/
pub const XKB_KEY_rightanglebracket             :u32 = 0x0abe;  /*(U+27E9 MATHEMATICAL RIGHT ANGLE BRACKET)*/
pub const XKB_KEY_marker                        :u32 = 0x0abf;
pub const XKB_KEY_oneeighth                     :u32 = 0x0ac3;  /* U+215B VULGAR FRACTION ONE EIGHTH */
pub const XKB_KEY_threeeighths                  :u32 = 0x0ac4;  /* U+215C VULGAR FRACTION THREE EIGHTHS */
pub const XKB_KEY_fiveeighths                   :u32 = 0x0ac5;  /* U+215D VULGAR FRACTION FIVE EIGHTHS */
pub const XKB_KEY_seveneighths                  :u32 = 0x0ac6;  /* U+215E VULGAR FRACTION SEVEN EIGHTHS */
pub const XKB_KEY_trademark                     :u32 = 0x0ac9;  /* U+2122 TRADE MARK SIGN */
pub const XKB_KEY_signaturemark                 :u32 = 0x0aca;  /*(U+2613 SALTIRE)*/
pub const XKB_KEY_trademarkincircle             :u32 = 0x0acb;
pub const XKB_KEY_leftopentriangle              :u32 = 0x0acc;  /*(U+25C1 WHITE LEFT-POINTING TRIANGLE)*/
pub const XKB_KEY_rightopentriangle             :u32 = 0x0acd;  /*(U+25B7 WHITE RIGHT-POINTING TRIANGLE)*/
pub const XKB_KEY_emopencircle                  :u32 = 0x0ace;  /*(U+25CB WHITE CIRCLE)*/
pub const XKB_KEY_emopenrectangle               :u32 = 0x0acf;  /*(U+25AF WHITE VERTICAL RECTANGLE)*/
pub const XKB_KEY_leftsinglequotemark           :u32 = 0x0ad0;  /* U+2018 LEFT SINGLE QUOTATION MARK */
pub const XKB_KEY_rightsinglequotemark          :u32 = 0x0ad1;  /* U+2019 RIGHT SINGLE QUOTATION MARK */
pub const XKB_KEY_leftdoublequotemark           :u32 = 0x0ad2;  /* U+201C LEFT DOUBLE QUOTATION MARK */
pub const XKB_KEY_rightdoublequotemark          :u32 = 0x0ad3;  /* U+201D RIGHT DOUBLE QUOTATION MARK */
pub const XKB_KEY_prescription                  :u32 = 0x0ad4;  /* U+211E PRESCRIPTION TAKE */
pub const XKB_KEY_permille                      :u32 = 0x0ad5;  /* U+2030 PER MILLE SIGN */
pub const XKB_KEY_minutes                       :u32 = 0x0ad6;  /* U+2032 PRIME */
pub const XKB_KEY_seconds                       :u32 = 0x0ad7;  /* U+2033 DOUBLE PRIME */
pub const XKB_KEY_latincross                    :u32 = 0x0ad9;  /* U+271D LATIN CROSS */
pub const XKB_KEY_hexagram                      :u32 = 0x0ada;
pub const XKB_KEY_filledrectbullet              :u32 = 0x0adb;  /*(U+25AC BLACK RECTANGLE)*/
pub const XKB_KEY_filledlefttribullet           :u32 = 0x0adc;  /*(U+25C0 BLACK LEFT-POINTING TRIANGLE)*/
pub const XKB_KEY_filledrighttribullet          :u32 = 0x0add;  /*(U+25B6 BLACK RIGHT-POINTING TRIANGLE)*/
pub const XKB_KEY_emfilledcircle                :u32 = 0x0ade;  /*(U+25CF BLACK CIRCLE)*/
pub const XKB_KEY_emfilledrect                  :u32 = 0x0adf;  /*(U+25AE BLACK VERTICAL RECTANGLE)*/
pub const XKB_KEY_enopencircbullet              :u32 = 0x0ae0;  /*(U+25E6 WHITE BULLET)*/
pub const XKB_KEY_enopensquarebullet            :u32 = 0x0ae1;  /*(U+25AB WHITE SMALL SQUARE)*/
pub const XKB_KEY_openrectbullet                :u32 = 0x0ae2;  /*(U+25AD WHITE RECTANGLE)*/
pub const XKB_KEY_opentribulletup               :u32 = 0x0ae3;  /*(U+25B3 WHITE UP-POINTING TRIANGLE)*/
pub const XKB_KEY_opentribulletdown             :u32 = 0x0ae4;  /*(U+25BD WHITE DOWN-POINTING TRIANGLE)*/
pub const XKB_KEY_openstar                      :u32 = 0x0ae5;  /*(U+2606 WHITE STAR)*/
pub const XKB_KEY_enfilledcircbullet            :u32 = 0x0ae6;  /*(U+2022 BULLET)*/
pub const XKB_KEY_enfilledsqbullet              :u32 = 0x0ae7;  /*(U+25AA BLACK SMALL SQUARE)*/
pub const XKB_KEY_filledtribulletup             :u32 = 0x0ae8;  /*(U+25B2 BLACK UP-POINTING TRIANGLE)*/
pub const XKB_KEY_filledtribulletdown           :u32 = 0x0ae9;  /*(U+25BC BLACK DOWN-POINTING TRIANGLE)*/
pub const XKB_KEY_leftpointer                   :u32 = 0x0aea;  /*(U+261C WHITE LEFT POINTING INDEX)*/
pub const XKB_KEY_rightpointer                  :u32 = 0x0aeb;  /*(U+261E WHITE RIGHT POINTING INDEX)*/
pub const XKB_KEY_club                          :u32 = 0x0aec;  /* U+2663 BLACK CLUB SUIT */
pub const XKB_KEY_diamond                       :u32 = 0x0aed;  /* U+2666 BLACK DIAMOND SUIT */
pub const XKB_KEY_heart                         :u32 = 0x0aee;  /* U+2665 BLACK HEART SUIT */
pub const XKB_KEY_maltesecross                  :u32 = 0x0af0;  /* U+2720 MALTESE CROSS */
pub const XKB_KEY_dagger                        :u32 = 0x0af1;  /* U+2020 DAGGER */
pub const XKB_KEY_doubledagger                  :u32 = 0x0af2;  /* U+2021 DOUBLE DAGGER */
pub const XKB_KEY_checkmark                     :u32 = 0x0af3;  /* U+2713 CHECK MARK */
pub const XKB_KEY_ballotcross                   :u32 = 0x0af4;  /* U+2717 BALLOT X */
pub const XKB_KEY_musicalsharp                  :u32 = 0x0af5;  /* U+266F MUSIC SHARP SIGN */
pub const XKB_KEY_musicalflat                   :u32 = 0x0af6;  /* U+266D MUSIC FLAT SIGN */
pub const XKB_KEY_malesymbol                    :u32 = 0x0af7;  /* U+2642 MALE SIGN */
pub const XKB_KEY_femalesymbol                  :u32 = 0x0af8;  /* U+2640 FEMALE SIGN */
pub const XKB_KEY_telephone                     :u32 = 0x0af9;  /* U+260E BLACK TELEPHONE */
pub const XKB_KEY_telephonerecorder             :u32 = 0x0afa;  /* U+2315 TELEPHONE RECORDER */
pub const XKB_KEY_phonographcopyright           :u32 = 0x0afb;  /* U+2117 SOUND RECORDING COPYRIGHT */
pub const XKB_KEY_caret                         :u32 = 0x0afc;  /* U+2038 CARET */
pub const XKB_KEY_singlelowquotemark            :u32 = 0x0afd;  /* U+201A SINGLE LOW-9 QUOTATION MARK */
pub const XKB_KEY_doublelowquotemark            :u32 = 0x0afe;  /* U+201E DOUBLE LOW-9 QUOTATION MARK */
pub const XKB_KEY_cursor                        :u32 = 0x0aff;

/*
 * APL
 * Byte 3 = :u32 = 0x0b;
 */

pub const XKB_KEY_leftcaret                     :u32 = 0x0ba3;  /*(U+003C LESS-THAN SIGN)*/
pub const XKB_KEY_rightcaret                    :u32 = 0x0ba6;  /*(U+003E GREATER-THAN SIGN)*/
pub const XKB_KEY_downcaret                     :u32 = 0x0ba8;  /*(U+2228 LOGICAL OR)*/
pub const XKB_KEY_upcaret                       :u32 = 0x0ba9;  /*(U+2227 LOGICAL AND)*/
pub const XKB_KEY_overbar                       :u32 = 0x0bc0;  /*(U+00AF MACRON)*/
pub const XKB_KEY_downtack                      :u32 = 0x0bc2;  /* U+22A4 DOWN TACK */
pub const XKB_KEY_upshoe                        :u32 = 0x0bc3;  /*(U+2229 INTERSECTION)*/
pub const XKB_KEY_downstile                     :u32 = 0x0bc4;  /* U+230A LEFT FLOOR */
pub const XKB_KEY_underbar                      :u32 = 0x0bc6;  /*(U+005F LOW LINE)*/
pub const XKB_KEY_jot                           :u32 = 0x0bca;  /* U+2218 RING OPERATOR */
pub const XKB_KEY_quad                          :u32 = 0x0bcc;  /* U+2395 APL FUNCTIONAL SYMBOL QUAD */
pub const XKB_KEY_uptack                        :u32 = 0x0bce;  /* U+22A5 UP TACK */
pub const XKB_KEY_circle                        :u32 = 0x0bcf;  /* U+25CB WHITE CIRCLE */
pub const XKB_KEY_upstile                       :u32 = 0x0bd3;  /* U+2308 LEFT CEILING */
pub const XKB_KEY_downshoe                      :u32 = 0x0bd6;  /*(U+222A UNION)*/
pub const XKB_KEY_rightshoe                     :u32 = 0x0bd8;  /*(U+2283 SUPERSET OF)*/
pub const XKB_KEY_leftshoe                      :u32 = 0x0bda;  /*(U+2282 SUBSET OF)*/
pub const XKB_KEY_lefttack                      :u32 = 0x0bdc;  /* U+22A3 LEFT TACK */
pub const XKB_KEY_righttack                     :u32 = 0x0bfc;  /* U+22A2 RIGHT TACK */

/*
 * Hebrew
 * Byte 3 = :u32 = 0x0c;
 */

pub const XKB_KEY_hebrew_doublelowline          :u32 = 0x0cdf;  /* U+2017 DOUBLE LOW LINE */
pub const XKB_KEY_hebrew_aleph                  :u32 = 0x0ce0;  /* U+05D0 HEBREW LETTER ALEF */
pub const XKB_KEY_hebrew_bet                    :u32 = 0x0ce1;  /* U+05D1 HEBREW LETTER BET */
pub const XKB_KEY_hebrew_beth                   :u32 = 0x0ce1;  /* deprecated */
pub const XKB_KEY_hebrew_gimel                  :u32 = 0x0ce2;  /* U+05D2 HEBREW LETTER GIMEL */
pub const XKB_KEY_hebrew_gimmel                 :u32 = 0x0ce2;  /* deprecated */
pub const XKB_KEY_hebrew_dalet                  :u32 = 0x0ce3;  /* U+05D3 HEBREW LETTER DALET */
pub const XKB_KEY_hebrew_daleth                 :u32 = 0x0ce3;  /* deprecated */
pub const XKB_KEY_hebrew_he                     :u32 = 0x0ce4;  /* U+05D4 HEBREW LETTER HE */
pub const XKB_KEY_hebrew_waw                    :u32 = 0x0ce5;  /* U+05D5 HEBREW LETTER VAV */
pub const XKB_KEY_hebrew_zain                   :u32 = 0x0ce6;  /* U+05D6 HEBREW LETTER ZAYIN */
pub const XKB_KEY_hebrew_zayin                  :u32 = 0x0ce6;  /* deprecated */
pub const XKB_KEY_hebrew_chet                   :u32 = 0x0ce7;  /* U+05D7 HEBREW LETTER HET */
pub const XKB_KEY_hebrew_het                    :u32 = 0x0ce7;  /* deprecated */
pub const XKB_KEY_hebrew_tet                    :u32 = 0x0ce8;  /* U+05D8 HEBREW LETTER TET */
pub const XKB_KEY_hebrew_teth                   :u32 = 0x0ce8;  /* deprecated */
pub const XKB_KEY_hebrew_yod                    :u32 = 0x0ce9;  /* U+05D9 HEBREW LETTER YOD */
pub const XKB_KEY_hebrew_finalkaph              :u32 = 0x0cea;  /* U+05DA HEBREW LETTER FINAL KAF */
pub const XKB_KEY_hebrew_kaph                   :u32 = 0x0ceb;  /* U+05DB HEBREW LETTER KAF */
pub const XKB_KEY_hebrew_lamed                  :u32 = 0x0cec;  /* U+05DC HEBREW LETTER LAMED */
pub const XKB_KEY_hebrew_finalmem               :u32 = 0x0ced;  /* U+05DD HEBREW LETTER FINAL MEM */
pub const XKB_KEY_hebrew_mem                    :u32 = 0x0cee;  /* U+05DE HEBREW LETTER MEM */
pub const XKB_KEY_hebrew_finalnun               :u32 = 0x0cef;  /* U+05DF HEBREW LETTER FINAL NUN */
pub const XKB_KEY_hebrew_nun                    :u32 = 0x0cf0;  /* U+05E0 HEBREW LETTER NUN */
pub const XKB_KEY_hebrew_samech                 :u32 = 0x0cf1;  /* U+05E1 HEBREW LETTER SAMEKH */
pub const XKB_KEY_hebrew_samekh                 :u32 = 0x0cf1;  /* deprecated */
pub const XKB_KEY_hebrew_ayin                   :u32 = 0x0cf2;  /* U+05E2 HEBREW LETTER AYIN */
pub const XKB_KEY_hebrew_finalpe                :u32 = 0x0cf3;  /* U+05E3 HEBREW LETTER FINAL PE */
pub const XKB_KEY_hebrew_pe                     :u32 = 0x0cf4;  /* U+05E4 HEBREW LETTER PE */
pub const XKB_KEY_hebrew_finalzade              :u32 = 0x0cf5;  /* U+05E5 HEBREW LETTER FINAL TSADI */
pub const XKB_KEY_hebrew_finalzadi              :u32 = 0x0cf5;  /* deprecated */
pub const XKB_KEY_hebrew_zade                   :u32 = 0x0cf6;  /* U+05E6 HEBREW LETTER TSADI */
pub const XKB_KEY_hebrew_zadi                   :u32 = 0x0cf6;  /* deprecated */
pub const XKB_KEY_hebrew_qoph                   :u32 = 0x0cf7;  /* U+05E7 HEBREW LETTER QOF */
pub const XKB_KEY_hebrew_kuf                    :u32 = 0x0cf7;  /* deprecated */
pub const XKB_KEY_hebrew_resh                   :u32 = 0x0cf8;  /* U+05E8 HEBREW LETTER RESH */
pub const XKB_KEY_hebrew_shin                   :u32 = 0x0cf9;  /* U+05E9 HEBREW LETTER SHIN */
pub const XKB_KEY_hebrew_taw                    :u32 = 0x0cfa;  /* U+05EA HEBREW LETTER TAV */
pub const XKB_KEY_hebrew_taf                    :u32 = 0x0cfa;  /* deprecated */
pub const XKB_KEY_Hebrew_switch                 :u32 = 0xff7e;  /* Alias for mode_switch */

/*
 * Thai
 * Byte 3 = :u32 = 0x0d;
 */

pub const XKB_KEY_Thai_kokai                    :u32 = 0x0da1;  /* U+0E01 THAI CHARACTER KO KAI */
pub const XKB_KEY_Thai_khokhai                  :u32 = 0x0da2;  /* U+0E02 THAI CHARACTER KHO KHAI */
pub const XKB_KEY_Thai_khokhuat                 :u32 = 0x0da3;  /* U+0E03 THAI CHARACTER KHO KHUAT */
pub const XKB_KEY_Thai_khokhwai                 :u32 = 0x0da4;  /* U+0E04 THAI CHARACTER KHO KHWAI */
pub const XKB_KEY_Thai_khokhon                  :u32 = 0x0da5;  /* U+0E05 THAI CHARACTER KHO KHON */
pub const XKB_KEY_Thai_khorakhang               :u32 = 0x0da6;  /* U+0E06 THAI CHARACTER KHO RAKHANG */
pub const XKB_KEY_Thai_ngongu                   :u32 = 0x0da7;  /* U+0E07 THAI CHARACTER NGO NGU */
pub const XKB_KEY_Thai_chochan                  :u32 = 0x0da8;  /* U+0E08 THAI CHARACTER CHO CHAN */
pub const XKB_KEY_Thai_choching                 :u32 = 0x0da9;  /* U+0E09 THAI CHARACTER CHO CHING */
pub const XKB_KEY_Thai_chochang                 :u32 = 0x0daa;  /* U+0E0A THAI CHARACTER CHO CHANG */
pub const XKB_KEY_Thai_soso                     :u32 = 0x0dab;  /* U+0E0B THAI CHARACTER SO SO */
pub const XKB_KEY_Thai_chochoe                  :u32 = 0x0dac;  /* U+0E0C THAI CHARACTER CHO CHOE */
pub const XKB_KEY_Thai_yoying                   :u32 = 0x0dad;  /* U+0E0D THAI CHARACTER YO YING */
pub const XKB_KEY_Thai_dochada                  :u32 = 0x0dae;  /* U+0E0E THAI CHARACTER DO CHADA */
pub const XKB_KEY_Thai_topatak                  :u32 = 0x0daf;  /* U+0E0F THAI CHARACTER TO PATAK */
pub const XKB_KEY_Thai_thothan                  :u32 = 0x0db0;  /* U+0E10 THAI CHARACTER THO THAN */
pub const XKB_KEY_Thai_thonangmontho            :u32 = 0x0db1;  /* U+0E11 THAI CHARACTER THO NANGMONTHO */
pub const XKB_KEY_Thai_thophuthao               :u32 = 0x0db2;  /* U+0E12 THAI CHARACTER THO PHUTHAO */
pub const XKB_KEY_Thai_nonen                    :u32 = 0x0db3;  /* U+0E13 THAI CHARACTER NO NEN */
pub const XKB_KEY_Thai_dodek                    :u32 = 0x0db4;  /* U+0E14 THAI CHARACTER DO DEK */
pub const XKB_KEY_Thai_totao                    :u32 = 0x0db5;  /* U+0E15 THAI CHARACTER TO TAO */
pub const XKB_KEY_Thai_thothung                 :u32 = 0x0db6;  /* U+0E16 THAI CHARACTER THO THUNG */
pub const XKB_KEY_Thai_thothahan                :u32 = 0x0db7;  /* U+0E17 THAI CHARACTER THO THAHAN */
pub const XKB_KEY_Thai_thothong                 :u32 = 0x0db8;  /* U+0E18 THAI CHARACTER THO THONG */
pub const XKB_KEY_Thai_nonu                     :u32 = 0x0db9;  /* U+0E19 THAI CHARACTER NO NU */
pub const XKB_KEY_Thai_bobaimai                 :u32 = 0x0dba;  /* U+0E1A THAI CHARACTER BO BAIMAI */
pub const XKB_KEY_Thai_popla                    :u32 = 0x0dbb;  /* U+0E1B THAI CHARACTER PO PLA */
pub const XKB_KEY_Thai_phophung                 :u32 = 0x0dbc;  /* U+0E1C THAI CHARACTER PHO PHUNG */
pub const XKB_KEY_Thai_fofa                     :u32 = 0x0dbd;  /* U+0E1D THAI CHARACTER FO FA */
pub const XKB_KEY_Thai_phophan                  :u32 = 0x0dbe;  /* U+0E1E THAI CHARACTER PHO PHAN */
pub const XKB_KEY_Thai_fofan                    :u32 = 0x0dbf;  /* U+0E1F THAI CHARACTER FO FAN */
pub const XKB_KEY_Thai_phosamphao               :u32 = 0x0dc0;  /* U+0E20 THAI CHARACTER PHO SAMPHAO */
pub const XKB_KEY_Thai_moma                     :u32 = 0x0dc1;  /* U+0E21 THAI CHARACTER MO MA */
pub const XKB_KEY_Thai_yoyak                    :u32 = 0x0dc2;  /* U+0E22 THAI CHARACTER YO YAK */
pub const XKB_KEY_Thai_rorua                    :u32 = 0x0dc3;  /* U+0E23 THAI CHARACTER RO RUA */
pub const XKB_KEY_Thai_ru                       :u32 = 0x0dc4;  /* U+0E24 THAI CHARACTER RU */
pub const XKB_KEY_Thai_loling                   :u32 = 0x0dc5;  /* U+0E25 THAI CHARACTER LO LING */
pub const XKB_KEY_Thai_lu                       :u32 = 0x0dc6;  /* U+0E26 THAI CHARACTER LU */
pub const XKB_KEY_Thai_wowaen                   :u32 = 0x0dc7;  /* U+0E27 THAI CHARACTER WO WAEN */
pub const XKB_KEY_Thai_sosala                   :u32 = 0x0dc8;  /* U+0E28 THAI CHARACTER SO SALA */
pub const XKB_KEY_Thai_sorusi                   :u32 = 0x0dc9;  /* U+0E29 THAI CHARACTER SO RUSI */
pub const XKB_KEY_Thai_sosua                    :u32 = 0x0dca;  /* U+0E2A THAI CHARACTER SO SUA */
pub const XKB_KEY_Thai_hohip                    :u32 = 0x0dcb;  /* U+0E2B THAI CHARACTER HO HIP */
pub const XKB_KEY_Thai_lochula                  :u32 = 0x0dcc;  /* U+0E2C THAI CHARACTER LO CHULA */
pub const XKB_KEY_Thai_oang                     :u32 = 0x0dcd;  /* U+0E2D THAI CHARACTER O ANG */
pub const XKB_KEY_Thai_honokhuk                 :u32 = 0x0dce;  /* U+0E2E THAI CHARACTER HO NOKHUK */
pub const XKB_KEY_Thai_paiyannoi                :u32 = 0x0dcf;  /* U+0E2F THAI CHARACTER PAIYANNOI */
pub const XKB_KEY_Thai_saraa                    :u32 = 0x0dd0;  /* U+0E30 THAI CHARACTER SARA A */
pub const XKB_KEY_Thai_maihanakat               :u32 = 0x0dd1;  /* U+0E31 THAI CHARACTER MAI HAN-AKAT */
pub const XKB_KEY_Thai_saraaa                   :u32 = 0x0dd2;  /* U+0E32 THAI CHARACTER SARA AA */
pub const XKB_KEY_Thai_saraam                   :u32 = 0x0dd3;  /* U+0E33 THAI CHARACTER SARA AM */
pub const XKB_KEY_Thai_sarai                    :u32 = 0x0dd4;  /* U+0E34 THAI CHARACTER SARA I */
pub const XKB_KEY_Thai_saraii                   :u32 = 0x0dd5;  /* U+0E35 THAI CHARACTER SARA II */
pub const XKB_KEY_Thai_saraue                   :u32 = 0x0dd6;  /* U+0E36 THAI CHARACTER SARA UE */
pub const XKB_KEY_Thai_sarauee                  :u32 = 0x0dd7;  /* U+0E37 THAI CHARACTER SARA UEE */
pub const XKB_KEY_Thai_sarau                    :u32 = 0x0dd8;  /* U+0E38 THAI CHARACTER SARA U */
pub const XKB_KEY_Thai_sarauu                   :u32 = 0x0dd9;  /* U+0E39 THAI CHARACTER SARA UU */
pub const XKB_KEY_Thai_phinthu                  :u32 = 0x0dda;  /* U+0E3A THAI CHARACTER PHINTHU */
pub const XKB_KEY_Thai_maihanakat_maitho        :u32 = 0x0dde;
pub const XKB_KEY_Thai_baht                     :u32 = 0x0ddf;  /* U+0E3F THAI CURRENCY SYMBOL BAHT */
pub const XKB_KEY_Thai_sarae                    :u32 = 0x0de0;  /* U+0E40 THAI CHARACTER SARA E */
pub const XKB_KEY_Thai_saraae                   :u32 = 0x0de1;  /* U+0E41 THAI CHARACTER SARA AE */
pub const XKB_KEY_Thai_sarao                    :u32 = 0x0de2;  /* U+0E42 THAI CHARACTER SARA O */
pub const XKB_KEY_Thai_saraaimaimuan            :u32 = 0x0de3;  /* U+0E43 THAI CHARACTER SARA AI MAIMUAN */
pub const XKB_KEY_Thai_saraaimaimalai           :u32 = 0x0de4;  /* U+0E44 THAI CHARACTER SARA AI MAIMALAI */
pub const XKB_KEY_Thai_lakkhangyao              :u32 = 0x0de5;  /* U+0E45 THAI CHARACTER LAKKHANGYAO */
pub const XKB_KEY_Thai_maiyamok                 :u32 = 0x0de6;  /* U+0E46 THAI CHARACTER MAIYAMOK */
pub const XKB_KEY_Thai_maitaikhu                :u32 = 0x0de7;  /* U+0E47 THAI CHARACTER MAITAIKHU */
pub const XKB_KEY_Thai_maiek                    :u32 = 0x0de8;  /* U+0E48 THAI CHARACTER MAI EK */
pub const XKB_KEY_Thai_maitho                   :u32 = 0x0de9;  /* U+0E49 THAI CHARACTER MAI THO */
pub const XKB_KEY_Thai_maitri                   :u32 = 0x0dea;  /* U+0E4A THAI CHARACTER MAI TRI */
pub const XKB_KEY_Thai_maichattawa              :u32 = 0x0deb;  /* U+0E4B THAI CHARACTER MAI CHATTAWA */
pub const XKB_KEY_Thai_thanthakhat              :u32 = 0x0dec;  /* U+0E4C THAI CHARACTER THANTHAKHAT */
pub const XKB_KEY_Thai_nikhahit                 :u32 = 0x0ded;  /* U+0E4D THAI CHARACTER NIKHAHIT */
pub const XKB_KEY_Thai_leksun                   :u32 = 0x0df0;  /* U+0E50 THAI DIGIT ZERO */
pub const XKB_KEY_Thai_leknung                  :u32 = 0x0df1;  /* U+0E51 THAI DIGIT ONE */
pub const XKB_KEY_Thai_leksong                  :u32 = 0x0df2;  /* U+0E52 THAI DIGIT TWO */
pub const XKB_KEY_Thai_leksam                   :u32 = 0x0df3;  /* U+0E53 THAI DIGIT THREE */
pub const XKB_KEY_Thai_leksi                    :u32 = 0x0df4;  /* U+0E54 THAI DIGIT FOUR */
pub const XKB_KEY_Thai_lekha                    :u32 = 0x0df5;  /* U+0E55 THAI DIGIT FIVE */
pub const XKB_KEY_Thai_lekhok                   :u32 = 0x0df6;  /* U+0E56 THAI DIGIT SIX */
pub const XKB_KEY_Thai_lekchet                  :u32 = 0x0df7;  /* U+0E57 THAI DIGIT SEVEN */
pub const XKB_KEY_Thai_lekpaet                  :u32 = 0x0df8;  /* U+0E58 THAI DIGIT EIGHT */
pub const XKB_KEY_Thai_lekkao                   :u32 = 0x0df9;  /* U+0E59 THAI DIGIT NINE */

/*
 * Korean
 * Byte 3 = :u32 = 0x0e;
 */


pub const XKB_KEY_Hangul                        :u32 = 0xff31;  /* Hangul start/stop(toggle) */
pub const XKB_KEY_Hangul_Start                  :u32 = 0xff32;  /* Hangul start */
pub const XKB_KEY_Hangul_End                    :u32 = 0xff33;  /* Hangul end, English start */
pub const XKB_KEY_Hangul_Hanja                  :u32 = 0xff34;  /* Start Hangul->Hanja Conversion */
pub const XKB_KEY_Hangul_Jamo                   :u32 = 0xff35;  /* Hangul Jamo mode */
pub const XKB_KEY_Hangul_Romaja                 :u32 = 0xff36;  /* Hangul Romaja mode */
pub const XKB_KEY_Hangul_Codeinput              :u32 = 0xff37;  /* Hangul code input mode */
pub const XKB_KEY_Hangul_Jeonja                 :u32 = 0xff38;  /* Jeonja mode */
pub const XKB_KEY_Hangul_Banja                  :u32 = 0xff39;  /* Banja mode */
pub const XKB_KEY_Hangul_PreHanja               :u32 = 0xff3a;  /* Pre Hanja conversion */
pub const XKB_KEY_Hangul_PostHanja              :u32 = 0xff3b;  /* Post Hanja conversion */
pub const XKB_KEY_Hangul_SingleCandidate        :u32 = 0xff3c;  /* Single candidate */
pub const XKB_KEY_Hangul_MultipleCandidate      :u32 = 0xff3d;  /* Multiple candidate */
pub const XKB_KEY_Hangul_PreviousCandidate      :u32 = 0xff3e;  /* Previous candidate */
pub const XKB_KEY_Hangul_Special                :u32 = 0xff3f;  /* Special symbols */
pub const XKB_KEY_Hangul_switch                 :u32 = 0xff7e;  /* Alias for mode_switch */

/* Hangul Consonant Characters */
pub const XKB_KEY_Hangul_Kiyeog                 :u32 = 0x0ea1;
pub const XKB_KEY_Hangul_SsangKiyeog            :u32 = 0x0ea2;
pub const XKB_KEY_Hangul_KiyeogSios             :u32 = 0x0ea3;
pub const XKB_KEY_Hangul_Nieun                  :u32 = 0x0ea4;
pub const XKB_KEY_Hangul_NieunJieuj             :u32 = 0x0ea5;
pub const XKB_KEY_Hangul_NieunHieuh             :u32 = 0x0ea6;
pub const XKB_KEY_Hangul_Dikeud                 :u32 = 0x0ea7;
pub const XKB_KEY_Hangul_SsangDikeud            :u32 = 0x0ea8;
pub const XKB_KEY_Hangul_Rieul                  :u32 = 0x0ea9;
pub const XKB_KEY_Hangul_RieulKiyeog            :u32 = 0x0eaa;
pub const XKB_KEY_Hangul_RieulMieum             :u32 = 0x0eab;
pub const XKB_KEY_Hangul_RieulPieub             :u32 = 0x0eac;
pub const XKB_KEY_Hangul_RieulSios              :u32 = 0x0ead;
pub const XKB_KEY_Hangul_RieulTieut             :u32 = 0x0eae;
pub const XKB_KEY_Hangul_RieulPhieuf            :u32 = 0x0eaf;
pub const XKB_KEY_Hangul_RieulHieuh             :u32 = 0x0eb0;
pub const XKB_KEY_Hangul_Mieum                  :u32 = 0x0eb1;
pub const XKB_KEY_Hangul_Pieub                  :u32 = 0x0eb2;
pub const XKB_KEY_Hangul_SsangPieub             :u32 = 0x0eb3;
pub const XKB_KEY_Hangul_PieubSios              :u32 = 0x0eb4;
pub const XKB_KEY_Hangul_Sios                   :u32 = 0x0eb5;
pub const XKB_KEY_Hangul_SsangSios              :u32 = 0x0eb6;
pub const XKB_KEY_Hangul_Ieung                  :u32 = 0x0eb7;
pub const XKB_KEY_Hangul_Jieuj                  :u32 = 0x0eb8;
pub const XKB_KEY_Hangul_SsangJieuj             :u32 = 0x0eb9;
pub const XKB_KEY_Hangul_Cieuc                  :u32 = 0x0eba;
pub const XKB_KEY_Hangul_Khieuq                 :u32 = 0x0ebb;
pub const XKB_KEY_Hangul_Tieut                  :u32 = 0x0ebc;
pub const XKB_KEY_Hangul_Phieuf                 :u32 = 0x0ebd;
pub const XKB_KEY_Hangul_Hieuh                  :u32 = 0x0ebe;

/* Hangul Vowel Characters */
pub const XKB_KEY_Hangul_A                      :u32 = 0x0ebf;
pub const XKB_KEY_Hangul_AE                     :u32 = 0x0ec0;
pub const XKB_KEY_Hangul_YA                     :u32 = 0x0ec1;
pub const XKB_KEY_Hangul_YAE                    :u32 = 0x0ec2;
pub const XKB_KEY_Hangul_EO                     :u32 = 0x0ec3;
pub const XKB_KEY_Hangul_E                      :u32 = 0x0ec4;
pub const XKB_KEY_Hangul_YEO                    :u32 = 0x0ec5;
pub const XKB_KEY_Hangul_YE                     :u32 = 0x0ec6;
pub const XKB_KEY_Hangul_O                      :u32 = 0x0ec7;
pub const XKB_KEY_Hangul_WA                     :u32 = 0x0ec8;
pub const XKB_KEY_Hangul_WAE                    :u32 = 0x0ec9;
pub const XKB_KEY_Hangul_OE                     :u32 = 0x0eca;
pub const XKB_KEY_Hangul_YO                     :u32 = 0x0ecb;
pub const XKB_KEY_Hangul_U                      :u32 = 0x0ecc;
pub const XKB_KEY_Hangul_WEO                    :u32 = 0x0ecd;
pub const XKB_KEY_Hangul_WE                     :u32 = 0x0ece;
pub const XKB_KEY_Hangul_WI                     :u32 = 0x0ecf;
pub const XKB_KEY_Hangul_YU                     :u32 = 0x0ed0;
pub const XKB_KEY_Hangul_EU                     :u32 = 0x0ed1;
pub const XKB_KEY_Hangul_YI                     :u32 = 0x0ed2;
pub const XKB_KEY_Hangul_I                      :u32 = 0x0ed3;

/* Hangul syllable-final (JongSeong) Characters */
pub const XKB_KEY_Hangul_J_Kiyeog               :u32 = 0x0ed4;
pub const XKB_KEY_Hangul_J_SsangKiyeog          :u32 = 0x0ed5;
pub const XKB_KEY_Hangul_J_KiyeogSios           :u32 = 0x0ed6;
pub const XKB_KEY_Hangul_J_Nieun                :u32 = 0x0ed7;
pub const XKB_KEY_Hangul_J_NieunJieuj           :u32 = 0x0ed8;
pub const XKB_KEY_Hangul_J_NieunHieuh           :u32 = 0x0ed9;
pub const XKB_KEY_Hangul_J_Dikeud               :u32 = 0x0eda;
pub const XKB_KEY_Hangul_J_Rieul                :u32 = 0x0edb;
pub const XKB_KEY_Hangul_J_RieulKiyeog          :u32 = 0x0edc;
pub const XKB_KEY_Hangul_J_RieulMieum           :u32 = 0x0edd;
pub const XKB_KEY_Hangul_J_RieulPieub           :u32 = 0x0ede;
pub const XKB_KEY_Hangul_J_RieulSios            :u32 = 0x0edf;
pub const XKB_KEY_Hangul_J_RieulTieut           :u32 = 0x0ee0;
pub const XKB_KEY_Hangul_J_RieulPhieuf          :u32 = 0x0ee1;
pub const XKB_KEY_Hangul_J_RieulHieuh           :u32 = 0x0ee2;
pub const XKB_KEY_Hangul_J_Mieum                :u32 = 0x0ee3;
pub const XKB_KEY_Hangul_J_Pieub                :u32 = 0x0ee4;
pub const XKB_KEY_Hangul_J_PieubSios            :u32 = 0x0ee5;
pub const XKB_KEY_Hangul_J_Sios                 :u32 = 0x0ee6;
pub const XKB_KEY_Hangul_J_SsangSios            :u32 = 0x0ee7;
pub const XKB_KEY_Hangul_J_Ieung                :u32 = 0x0ee8;
pub const XKB_KEY_Hangul_J_Jieuj                :u32 = 0x0ee9;
pub const XKB_KEY_Hangul_J_Cieuc                :u32 = 0x0eea;
pub const XKB_KEY_Hangul_J_Khieuq               :u32 = 0x0eeb;
pub const XKB_KEY_Hangul_J_Tieut                :u32 = 0x0eec;
pub const XKB_KEY_Hangul_J_Phieuf               :u32 = 0x0eed;
pub const XKB_KEY_Hangul_J_Hieuh                :u32 = 0x0eee;

/* Ancient Hangul Consonant Characters */
pub const XKB_KEY_Hangul_RieulYeorinHieuh       :u32 = 0x0eef;
pub const XKB_KEY_Hangul_SunkyeongeumMieum      :u32 = 0x0ef0;
pub const XKB_KEY_Hangul_SunkyeongeumPieub      :u32 = 0x0ef1;
pub const XKB_KEY_Hangul_PanSios                :u32 = 0x0ef2;
pub const XKB_KEY_Hangul_KkogjiDalrinIeung      :u32 = 0x0ef3;
pub const XKB_KEY_Hangul_SunkyeongeumPhieuf     :u32 = 0x0ef4;
pub const XKB_KEY_Hangul_YeorinHieuh            :u32 = 0x0ef5;

/* Ancient Hangul Vowel Characters */
pub const XKB_KEY_Hangul_AraeA                  :u32 = 0x0ef6;
pub const XKB_KEY_Hangul_AraeAE                 :u32 = 0x0ef7;

/* Ancient Hangul syllable-final (JongSeong) Characters */
pub const XKB_KEY_Hangul_J_PanSios              :u32 = 0x0ef8;
pub const XKB_KEY_Hangul_J_KkogjiDalrinIeung    :u32 = 0x0ef9;
pub const XKB_KEY_Hangul_J_YeorinHieuh          :u32 = 0x0efa;

/* Korean currency symbol */
pub const XKB_KEY_Korean_Won                    :u32 = 0x0eff;  /*(U+20A9 WON SIGN)*/


/*
 * Armenian
 */

pub const XKB_KEY_Armenian_ligature_ew       :u32 = 0x1000587;  /* U+0587 ARMENIAN SMALL LIGATURE ECH YIWN */
pub const XKB_KEY_Armenian_full_stop         :u32 = 0x1000589;  /* U+0589 ARMENIAN FULL STOP */
pub const XKB_KEY_Armenian_verjaket          :u32 = 0x1000589;  /* U+0589 ARMENIAN FULL STOP */
pub const XKB_KEY_Armenian_separation_mark   :u32 = 0x100055d;  /* U+055D ARMENIAN COMMA */
pub const XKB_KEY_Armenian_but               :u32 = 0x100055d;  /* U+055D ARMENIAN COMMA */
pub const XKB_KEY_Armenian_hyphen            :u32 = 0x100058a;  /* U+058A ARMENIAN HYPHEN */
pub const XKB_KEY_Armenian_yentamna          :u32 = 0x100058a;  /* U+058A ARMENIAN HYPHEN */
pub const XKB_KEY_Armenian_exclam            :u32 = 0x100055c;  /* U+055C ARMENIAN EXCLAMATION MARK */
pub const XKB_KEY_Armenian_amanak            :u32 = 0x100055c;  /* U+055C ARMENIAN EXCLAMATION MARK */
pub const XKB_KEY_Armenian_accent            :u32 = 0x100055b;  /* U+055B ARMENIAN EMPHASIS MARK */
pub const XKB_KEY_Armenian_shesht            :u32 = 0x100055b;  /* U+055B ARMENIAN EMPHASIS MARK */
pub const XKB_KEY_Armenian_question          :u32 = 0x100055e;  /* U+055E ARMENIAN QUESTION MARK */
pub const XKB_KEY_Armenian_paruyk            :u32 = 0x100055e;  /* U+055E ARMENIAN QUESTION MARK */
pub const XKB_KEY_Armenian_AYB               :u32 = 0x1000531;  /* U+0531 ARMENIAN CAPITAL LETTER AYB */
pub const XKB_KEY_Armenian_ayb               :u32 = 0x1000561;  /* U+0561 ARMENIAN SMALL LETTER AYB */
pub const XKB_KEY_Armenian_BEN               :u32 = 0x1000532;  /* U+0532 ARMENIAN CAPITAL LETTER BEN */
pub const XKB_KEY_Armenian_ben               :u32 = 0x1000562;  /* U+0562 ARMENIAN SMALL LETTER BEN */
pub const XKB_KEY_Armenian_GIM               :u32 = 0x1000533;  /* U+0533 ARMENIAN CAPITAL LETTER GIM */
pub const XKB_KEY_Armenian_gim               :u32 = 0x1000563;  /* U+0563 ARMENIAN SMALL LETTER GIM */
pub const XKB_KEY_Armenian_DA                :u32 = 0x1000534;  /* U+0534 ARMENIAN CAPITAL LETTER DA */
pub const XKB_KEY_Armenian_da                :u32 = 0x1000564;  /* U+0564 ARMENIAN SMALL LETTER DA */
pub const XKB_KEY_Armenian_YECH              :u32 = 0x1000535;  /* U+0535 ARMENIAN CAPITAL LETTER ECH */
pub const XKB_KEY_Armenian_yech              :u32 = 0x1000565;  /* U+0565 ARMENIAN SMALL LETTER ECH */
pub const XKB_KEY_Armenian_ZA                :u32 = 0x1000536;  /* U+0536 ARMENIAN CAPITAL LETTER ZA */
pub const XKB_KEY_Armenian_za                :u32 = 0x1000566;  /* U+0566 ARMENIAN SMALL LETTER ZA */
pub const XKB_KEY_Armenian_E                 :u32 = 0x1000537;  /* U+0537 ARMENIAN CAPITAL LETTER EH */
pub const XKB_KEY_Armenian_e                 :u32 = 0x1000567;  /* U+0567 ARMENIAN SMALL LETTER EH */
pub const XKB_KEY_Armenian_AT                :u32 = 0x1000538;  /* U+0538 ARMENIAN CAPITAL LETTER ET */
pub const XKB_KEY_Armenian_at                :u32 = 0x1000568;  /* U+0568 ARMENIAN SMALL LETTER ET */
pub const XKB_KEY_Armenian_TO                :u32 = 0x1000539;  /* U+0539 ARMENIAN CAPITAL LETTER TO */
pub const XKB_KEY_Armenian_to                :u32 = 0x1000569;  /* U+0569 ARMENIAN SMALL LETTER TO */
pub const XKB_KEY_Armenian_ZHE               :u32 = 0x100053a;  /* U+053A ARMENIAN CAPITAL LETTER ZHE */
pub const XKB_KEY_Armenian_zhe               :u32 = 0x100056a;  /* U+056A ARMENIAN SMALL LETTER ZHE */
pub const XKB_KEY_Armenian_INI               :u32 = 0x100053b;  /* U+053B ARMENIAN CAPITAL LETTER INI */
pub const XKB_KEY_Armenian_ini               :u32 = 0x100056b;  /* U+056B ARMENIAN SMALL LETTER INI */
pub const XKB_KEY_Armenian_LYUN              :u32 = 0x100053c;  /* U+053C ARMENIAN CAPITAL LETTER LIWN */
pub const XKB_KEY_Armenian_lyun              :u32 = 0x100056c;  /* U+056C ARMENIAN SMALL LETTER LIWN */
pub const XKB_KEY_Armenian_KHE               :u32 = 0x100053d;  /* U+053D ARMENIAN CAPITAL LETTER XEH */
pub const XKB_KEY_Armenian_khe               :u32 = 0x100056d;  /* U+056D ARMENIAN SMALL LETTER XEH */
pub const XKB_KEY_Armenian_TSA               :u32 = 0x100053e;  /* U+053E ARMENIAN CAPITAL LETTER CA */
pub const XKB_KEY_Armenian_tsa               :u32 = 0x100056e;  /* U+056E ARMENIAN SMALL LETTER CA */
pub const XKB_KEY_Armenian_KEN               :u32 = 0x100053f;  /* U+053F ARMENIAN CAPITAL LETTER KEN */
pub const XKB_KEY_Armenian_ken               :u32 = 0x100056f;  /* U+056F ARMENIAN SMALL LETTER KEN */
pub const XKB_KEY_Armenian_HO                :u32 = 0x1000540;  /* U+0540 ARMENIAN CAPITAL LETTER HO */
pub const XKB_KEY_Armenian_ho                :u32 = 0x1000570;  /* U+0570 ARMENIAN SMALL LETTER HO */
pub const XKB_KEY_Armenian_DZA               :u32 = 0x1000541;  /* U+0541 ARMENIAN CAPITAL LETTER JA */
pub const XKB_KEY_Armenian_dza               :u32 = 0x1000571;  /* U+0571 ARMENIAN SMALL LETTER JA */
pub const XKB_KEY_Armenian_GHAT              :u32 = 0x1000542;  /* U+0542 ARMENIAN CAPITAL LETTER GHAD */
pub const XKB_KEY_Armenian_ghat              :u32 = 0x1000572;  /* U+0572 ARMENIAN SMALL LETTER GHAD */
pub const XKB_KEY_Armenian_TCHE              :u32 = 0x1000543;  /* U+0543 ARMENIAN CAPITAL LETTER CHEH */
pub const XKB_KEY_Armenian_tche              :u32 = 0x1000573;  /* U+0573 ARMENIAN SMALL LETTER CHEH */
pub const XKB_KEY_Armenian_MEN               :u32 = 0x1000544;  /* U+0544 ARMENIAN CAPITAL LETTER MEN */
pub const XKB_KEY_Armenian_men               :u32 = 0x1000574;  /* U+0574 ARMENIAN SMALL LETTER MEN */
pub const XKB_KEY_Armenian_HI                :u32 = 0x1000545;  /* U+0545 ARMENIAN CAPITAL LETTER YI */
pub const XKB_KEY_Armenian_hi                :u32 = 0x1000575;  /* U+0575 ARMENIAN SMALL LETTER YI */
pub const XKB_KEY_Armenian_NU                :u32 = 0x1000546;  /* U+0546 ARMENIAN CAPITAL LETTER NOW */
pub const XKB_KEY_Armenian_nu                :u32 = 0x1000576;  /* U+0576 ARMENIAN SMALL LETTER NOW */
pub const XKB_KEY_Armenian_SHA               :u32 = 0x1000547;  /* U+0547 ARMENIAN CAPITAL LETTER SHA */
pub const XKB_KEY_Armenian_sha               :u32 = 0x1000577;  /* U+0577 ARMENIAN SMALL LETTER SHA */
pub const XKB_KEY_Armenian_VO                :u32 = 0x1000548;  /* U+0548 ARMENIAN CAPITAL LETTER VO */
pub const XKB_KEY_Armenian_vo                :u32 = 0x1000578;  /* U+0578 ARMENIAN SMALL LETTER VO */
pub const XKB_KEY_Armenian_CHA               :u32 = 0x1000549;  /* U+0549 ARMENIAN CAPITAL LETTER CHA */
pub const XKB_KEY_Armenian_cha               :u32 = 0x1000579;  /* U+0579 ARMENIAN SMALL LETTER CHA */
pub const XKB_KEY_Armenian_PE                :u32 = 0x100054a;  /* U+054A ARMENIAN CAPITAL LETTER PEH */
pub const XKB_KEY_Armenian_pe                :u32 = 0x100057a;  /* U+057A ARMENIAN SMALL LETTER PEH */
pub const XKB_KEY_Armenian_JE                :u32 = 0x100054b;  /* U+054B ARMENIAN CAPITAL LETTER JHEH */
pub const XKB_KEY_Armenian_je                :u32 = 0x100057b;  /* U+057B ARMENIAN SMALL LETTER JHEH */
pub const XKB_KEY_Armenian_RA                :u32 = 0x100054c;  /* U+054C ARMENIAN CAPITAL LETTER RA */
pub const XKB_KEY_Armenian_ra                :u32 = 0x100057c;  /* U+057C ARMENIAN SMALL LETTER RA */
pub const XKB_KEY_Armenian_SE                :u32 = 0x100054d;  /* U+054D ARMENIAN CAPITAL LETTER SEH */
pub const XKB_KEY_Armenian_se                :u32 = 0x100057d;  /* U+057D ARMENIAN SMALL LETTER SEH */
pub const XKB_KEY_Armenian_VEV               :u32 = 0x100054e;  /* U+054E ARMENIAN CAPITAL LETTER VEW */
pub const XKB_KEY_Armenian_vev               :u32 = 0x100057e;  /* U+057E ARMENIAN SMALL LETTER VEW */
pub const XKB_KEY_Armenian_TYUN              :u32 = 0x100054f;  /* U+054F ARMENIAN CAPITAL LETTER TIWN */
pub const XKB_KEY_Armenian_tyun              :u32 = 0x100057f;  /* U+057F ARMENIAN SMALL LETTER TIWN */
pub const XKB_KEY_Armenian_RE                :u32 = 0x1000550;  /* U+0550 ARMENIAN CAPITAL LETTER REH */
pub const XKB_KEY_Armenian_re                :u32 = 0x1000580;  /* U+0580 ARMENIAN SMALL LETTER REH */
pub const XKB_KEY_Armenian_TSO               :u32 = 0x1000551;  /* U+0551 ARMENIAN CAPITAL LETTER CO */
pub const XKB_KEY_Armenian_tso               :u32 = 0x1000581;  /* U+0581 ARMENIAN SMALL LETTER CO */
pub const XKB_KEY_Armenian_VYUN              :u32 = 0x1000552;  /* U+0552 ARMENIAN CAPITAL LETTER YIWN */
pub const XKB_KEY_Armenian_vyun              :u32 = 0x1000582;  /* U+0582 ARMENIAN SMALL LETTER YIWN */
pub const XKB_KEY_Armenian_PYUR              :u32 = 0x1000553;  /* U+0553 ARMENIAN CAPITAL LETTER PIWR */
pub const XKB_KEY_Armenian_pyur              :u32 = 0x1000583;  /* U+0583 ARMENIAN SMALL LETTER PIWR */
pub const XKB_KEY_Armenian_KE                :u32 = 0x1000554;  /* U+0554 ARMENIAN CAPITAL LETTER KEH */
pub const XKB_KEY_Armenian_ke                :u32 = 0x1000584;  /* U+0584 ARMENIAN SMALL LETTER KEH */
pub const XKB_KEY_Armenian_O                 :u32 = 0x1000555;  /* U+0555 ARMENIAN CAPITAL LETTER OH */
pub const XKB_KEY_Armenian_o                 :u32 = 0x1000585;  /* U+0585 ARMENIAN SMALL LETTER OH */
pub const XKB_KEY_Armenian_FE                :u32 = 0x1000556;  /* U+0556 ARMENIAN CAPITAL LETTER FEH */
pub const XKB_KEY_Armenian_fe                :u32 = 0x1000586;  /* U+0586 ARMENIAN SMALL LETTER FEH */
pub const XKB_KEY_Armenian_apostrophe        :u32 = 0x100055a;  /* U+055A ARMENIAN APOSTROPHE */

/*
 * Georgian
 */

pub const XKB_KEY_Georgian_an                :u32 = 0x10010d0;  /* U+10D0 GEORGIAN LETTER AN */
pub const XKB_KEY_Georgian_ban               :u32 = 0x10010d1;  /* U+10D1 GEORGIAN LETTER BAN */
pub const XKB_KEY_Georgian_gan               :u32 = 0x10010d2;  /* U+10D2 GEORGIAN LETTER GAN */
pub const XKB_KEY_Georgian_don               :u32 = 0x10010d3;  /* U+10D3 GEORGIAN LETTER DON */
pub const XKB_KEY_Georgian_en                :u32 = 0x10010d4;  /* U+10D4 GEORGIAN LETTER EN */
pub const XKB_KEY_Georgian_vin               :u32 = 0x10010d5;  /* U+10D5 GEORGIAN LETTER VIN */
pub const XKB_KEY_Georgian_zen               :u32 = 0x10010d6;  /* U+10D6 GEORGIAN LETTER ZEN */
pub const XKB_KEY_Georgian_tan               :u32 = 0x10010d7;  /* U+10D7 GEORGIAN LETTER TAN */
pub const XKB_KEY_Georgian_in                :u32 = 0x10010d8;  /* U+10D8 GEORGIAN LETTER IN */
pub const XKB_KEY_Georgian_kan               :u32 = 0x10010d9;  /* U+10D9 GEORGIAN LETTER KAN */
pub const XKB_KEY_Georgian_las               :u32 = 0x10010da;  /* U+10DA GEORGIAN LETTER LAS */
pub const XKB_KEY_Georgian_man               :u32 = 0x10010db;  /* U+10DB GEORGIAN LETTER MAN */
pub const XKB_KEY_Georgian_nar               :u32 = 0x10010dc;  /* U+10DC GEORGIAN LETTER NAR */
pub const XKB_KEY_Georgian_on                :u32 = 0x10010dd;  /* U+10DD GEORGIAN LETTER ON */
pub const XKB_KEY_Georgian_par               :u32 = 0x10010de;  /* U+10DE GEORGIAN LETTER PAR */
pub const XKB_KEY_Georgian_zhar              :u32 = 0x10010df;  /* U+10DF GEORGIAN LETTER ZHAR */
pub const XKB_KEY_Georgian_rae               :u32 = 0x10010e0;  /* U+10E0 GEORGIAN LETTER RAE */
pub const XKB_KEY_Georgian_san               :u32 = 0x10010e1;  /* U+10E1 GEORGIAN LETTER SAN */
pub const XKB_KEY_Georgian_tar               :u32 = 0x10010e2;  /* U+10E2 GEORGIAN LETTER TAR */
pub const XKB_KEY_Georgian_un                :u32 = 0x10010e3;  /* U+10E3 GEORGIAN LETTER UN */
pub const XKB_KEY_Georgian_phar              :u32 = 0x10010e4;  /* U+10E4 GEORGIAN LETTER PHAR */
pub const XKB_KEY_Georgian_khar              :u32 = 0x10010e5;  /* U+10E5 GEORGIAN LETTER KHAR */
pub const XKB_KEY_Georgian_ghan              :u32 = 0x10010e6;  /* U+10E6 GEORGIAN LETTER GHAN */
pub const XKB_KEY_Georgian_qar               :u32 = 0x10010e7;  /* U+10E7 GEORGIAN LETTER QAR */
pub const XKB_KEY_Georgian_shin              :u32 = 0x10010e8;  /* U+10E8 GEORGIAN LETTER SHIN */
pub const XKB_KEY_Georgian_chin              :u32 = 0x10010e9;  /* U+10E9 GEORGIAN LETTER CHIN */
pub const XKB_KEY_Georgian_can               :u32 = 0x10010ea;  /* U+10EA GEORGIAN LETTER CAN */
pub const XKB_KEY_Georgian_jil               :u32 = 0x10010eb;  /* U+10EB GEORGIAN LETTER JIL */
pub const XKB_KEY_Georgian_cil               :u32 = 0x10010ec;  /* U+10EC GEORGIAN LETTER CIL */
pub const XKB_KEY_Georgian_char              :u32 = 0x10010ed;  /* U+10ED GEORGIAN LETTER CHAR */
pub const XKB_KEY_Georgian_xan               :u32 = 0x10010ee;  /* U+10EE GEORGIAN LETTER XAN */
pub const XKB_KEY_Georgian_jhan              :u32 = 0x10010ef;  /* U+10EF GEORGIAN LETTER JHAN */
pub const XKB_KEY_Georgian_hae               :u32 = 0x10010f0;  /* U+10F0 GEORGIAN LETTER HAE */
pub const XKB_KEY_Georgian_he                :u32 = 0x10010f1;  /* U+10F1 GEORGIAN LETTER HE */
pub const XKB_KEY_Georgian_hie               :u32 = 0x10010f2;  /* U+10F2 GEORGIAN LETTER HIE */
pub const XKB_KEY_Georgian_we                :u32 = 0x10010f3;  /* U+10F3 GEORGIAN LETTER WE */
pub const XKB_KEY_Georgian_har               :u32 = 0x10010f4;  /* U+10F4 GEORGIAN LETTER HAR */
pub const XKB_KEY_Georgian_hoe               :u32 = 0x10010f5;  /* U+10F5 GEORGIAN LETTER HOE */
pub const XKB_KEY_Georgian_fi                :u32 = 0x10010f6;  /* U+10F6 GEORGIAN LETTER FI */

/*
 * Azeri (and other Turkic or Caucasian languages)
 */

/* latin */
pub const XKB_KEY_Xabovedot                  :u32 = 0x1001e8a;  /* U+1E8A LATIN CAPITAL LETTER X WITH DOT ABOVE */
pub const XKB_KEY_Ibreve                     :u32 = 0x100012c;  /* U+012C LATIN CAPITAL LETTER I WITH BREVE */
pub const XKB_KEY_Zstroke                    :u32 = 0x10001b5;  /* U+01B5 LATIN CAPITAL LETTER Z WITH STROKE */
pub const XKB_KEY_Gcaron                     :u32 = 0x10001e6;  /* U+01E6 LATIN CAPITAL LETTER G WITH CARON */
pub const XKB_KEY_Ocaron                     :u32 = 0x10001d1;  /* U+01D1 LATIN CAPITAL LETTER O WITH CARON */
pub const XKB_KEY_Obarred                    :u32 = 0x100019f;  /* U+019F LATIN CAPITAL LETTER O WITH MIDDLE TILDE */
pub const XKB_KEY_xabovedot                  :u32 = 0x1001e8b;  /* U+1E8B LATIN SMALL LETTER X WITH DOT ABOVE */
pub const XKB_KEY_ibreve                     :u32 = 0x100012d;  /* U+012D LATIN SMALL LETTER I WITH BREVE */
pub const XKB_KEY_zstroke                    :u32 = 0x10001b6;  /* U+01B6 LATIN SMALL LETTER Z WITH STROKE */
pub const XKB_KEY_gcaron                     :u32 = 0x10001e7;  /* U+01E7 LATIN SMALL LETTER G WITH CARON */
pub const XKB_KEY_ocaron                     :u32 = 0x10001d2;  /* U+01D2 LATIN SMALL LETTER O WITH CARON */
pub const XKB_KEY_obarred                    :u32 = 0x1000275;  /* U+0275 LATIN SMALL LETTER BARRED O */
pub const XKB_KEY_SCHWA                      :u32 = 0x100018f;  /* U+018F LATIN CAPITAL LETTER SCHWA */
pub const XKB_KEY_schwa                      :u32 = 0x1000259;  /* U+0259 LATIN SMALL LETTER SCHWA */
pub const XKB_KEY_EZH                        :u32 = 0x10001b7;  /* U+01B7 LATIN CAPITAL LETTER EZH */
pub const XKB_KEY_ezh                        :u32 = 0x1000292;  /* U+0292 LATIN SMALL LETTER EZH */
/* those are not really Caucasus */
/* For Inupiak */
pub const XKB_KEY_Lbelowdot                  :u32 = 0x1001e36;  /* U+1E36 LATIN CAPITAL LETTER L WITH DOT BELOW */
pub const XKB_KEY_lbelowdot                  :u32 = 0x1001e37;  /* U+1E37 LATIN SMALL LETTER L WITH DOT BELOW */

/*
 * Vietnamese
 */

pub const XKB_KEY_Abelowdot                  :u32 = 0x1001ea0;  /* U+1EA0 LATIN CAPITAL LETTER A WITH DOT BELOW */
pub const XKB_KEY_abelowdot                  :u32 = 0x1001ea1;  /* U+1EA1 LATIN SMALL LETTER A WITH DOT BELOW */
pub const XKB_KEY_Ahook                      :u32 = 0x1001ea2;  /* U+1EA2 LATIN CAPITAL LETTER A WITH HOOK ABOVE */
pub const XKB_KEY_ahook                      :u32 = 0x1001ea3;  /* U+1EA3 LATIN SMALL LETTER A WITH HOOK ABOVE */
pub const XKB_KEY_Acircumflexacute           :u32 = 0x1001ea4;  /* U+1EA4 LATIN CAPITAL LETTER A WITH CIRCUMFLEX AND ACUTE */
pub const XKB_KEY_acircumflexacute           :u32 = 0x1001ea5;  /* U+1EA5 LATIN SMALL LETTER A WITH CIRCUMFLEX AND ACUTE */
pub const XKB_KEY_Acircumflexgrave           :u32 = 0x1001ea6;  /* U+1EA6 LATIN CAPITAL LETTER A WITH CIRCUMFLEX AND GRAVE */
pub const XKB_KEY_acircumflexgrave           :u32 = 0x1001ea7;  /* U+1EA7 LATIN SMALL LETTER A WITH CIRCUMFLEX AND GRAVE */
pub const XKB_KEY_Acircumflexhook            :u32 = 0x1001ea8;  /* U+1EA8 LATIN CAPITAL LETTER A WITH CIRCUMFLEX AND HOOK ABOVE */
pub const XKB_KEY_acircumflexhook            :u32 = 0x1001ea9;  /* U+1EA9 LATIN SMALL LETTER A WITH CIRCUMFLEX AND HOOK ABOVE */
pub const XKB_KEY_Acircumflextilde           :u32 = 0x1001eaa;  /* U+1EAA LATIN CAPITAL LETTER A WITH CIRCUMFLEX AND TILDE */
pub const XKB_KEY_acircumflextilde           :u32 = 0x1001eab;  /* U+1EAB LATIN SMALL LETTER A WITH CIRCUMFLEX AND TILDE */
pub const XKB_KEY_Acircumflexbelowdot        :u32 = 0x1001eac;  /* U+1EAC LATIN CAPITAL LETTER A WITH CIRCUMFLEX AND DOT BELOW */
pub const XKB_KEY_acircumflexbelowdot        :u32 = 0x1001ead;  /* U+1EAD LATIN SMALL LETTER A WITH CIRCUMFLEX AND DOT BELOW */
pub const XKB_KEY_Abreveacute                :u32 = 0x1001eae;  /* U+1EAE LATIN CAPITAL LETTER A WITH BREVE AND ACUTE */
pub const XKB_KEY_abreveacute                :u32 = 0x1001eaf;  /* U+1EAF LATIN SMALL LETTER A WITH BREVE AND ACUTE */
pub const XKB_KEY_Abrevegrave                :u32 = 0x1001eb0;  /* U+1EB0 LATIN CAPITAL LETTER A WITH BREVE AND GRAVE */
pub const XKB_KEY_abrevegrave                :u32 = 0x1001eb1;  /* U+1EB1 LATIN SMALL LETTER A WITH BREVE AND GRAVE */
pub const XKB_KEY_Abrevehook                 :u32 = 0x1001eb2;  /* U+1EB2 LATIN CAPITAL LETTER A WITH BREVE AND HOOK ABOVE */
pub const XKB_KEY_abrevehook                 :u32 = 0x1001eb3;  /* U+1EB3 LATIN SMALL LETTER A WITH BREVE AND HOOK ABOVE */
pub const XKB_KEY_Abrevetilde                :u32 = 0x1001eb4;  /* U+1EB4 LATIN CAPITAL LETTER A WITH BREVE AND TILDE */
pub const XKB_KEY_abrevetilde                :u32 = 0x1001eb5;  /* U+1EB5 LATIN SMALL LETTER A WITH BREVE AND TILDE */
pub const XKB_KEY_Abrevebelowdot             :u32 = 0x1001eb6;  /* U+1EB6 LATIN CAPITAL LETTER A WITH BREVE AND DOT BELOW */
pub const XKB_KEY_abrevebelowdot             :u32 = 0x1001eb7;  /* U+1EB7 LATIN SMALL LETTER A WITH BREVE AND DOT BELOW */
pub const XKB_KEY_Ebelowdot                  :u32 = 0x1001eb8;  /* U+1EB8 LATIN CAPITAL LETTER E WITH DOT BELOW */
pub const XKB_KEY_ebelowdot                  :u32 = 0x1001eb9;  /* U+1EB9 LATIN SMALL LETTER E WITH DOT BELOW */
pub const XKB_KEY_Ehook                      :u32 = 0x1001eba;  /* U+1EBA LATIN CAPITAL LETTER E WITH HOOK ABOVE */
pub const XKB_KEY_ehook                      :u32 = 0x1001ebb;  /* U+1EBB LATIN SMALL LETTER E WITH HOOK ABOVE */
pub const XKB_KEY_Etilde                     :u32 = 0x1001ebc;  /* U+1EBC LATIN CAPITAL LETTER E WITH TILDE */
pub const XKB_KEY_etilde                     :u32 = 0x1001ebd;  /* U+1EBD LATIN SMALL LETTER E WITH TILDE */
pub const XKB_KEY_Ecircumflexacute           :u32 = 0x1001ebe;  /* U+1EBE LATIN CAPITAL LETTER E WITH CIRCUMFLEX AND ACUTE */
pub const XKB_KEY_ecircumflexacute           :u32 = 0x1001ebf;  /* U+1EBF LATIN SMALL LETTER E WITH CIRCUMFLEX AND ACUTE */
pub const XKB_KEY_Ecircumflexgrave           :u32 = 0x1001ec0;  /* U+1EC0 LATIN CAPITAL LETTER E WITH CIRCUMFLEX AND GRAVE */
pub const XKB_KEY_ecircumflexgrave           :u32 = 0x1001ec1;  /* U+1EC1 LATIN SMALL LETTER E WITH CIRCUMFLEX AND GRAVE */
pub const XKB_KEY_Ecircumflexhook            :u32 = 0x1001ec2;  /* U+1EC2 LATIN CAPITAL LETTER E WITH CIRCUMFLEX AND HOOK ABOVE */
pub const XKB_KEY_ecircumflexhook            :u32 = 0x1001ec3;  /* U+1EC3 LATIN SMALL LETTER E WITH CIRCUMFLEX AND HOOK ABOVE */
pub const XKB_KEY_Ecircumflextilde           :u32 = 0x1001ec4;  /* U+1EC4 LATIN CAPITAL LETTER E WITH CIRCUMFLEX AND TILDE */
pub const XKB_KEY_ecircumflextilde           :u32 = 0x1001ec5;  /* U+1EC5 LATIN SMALL LETTER E WITH CIRCUMFLEX AND TILDE */
pub const XKB_KEY_Ecircumflexbelowdot        :u32 = 0x1001ec6;  /* U+1EC6 LATIN CAPITAL LETTER E WITH CIRCUMFLEX AND DOT BELOW */
pub const XKB_KEY_ecircumflexbelowdot        :u32 = 0x1001ec7;  /* U+1EC7 LATIN SMALL LETTER E WITH CIRCUMFLEX AND DOT BELOW */
pub const XKB_KEY_Ihook                      :u32 = 0x1001ec8;  /* U+1EC8 LATIN CAPITAL LETTER I WITH HOOK ABOVE */
pub const XKB_KEY_ihook                      :u32 = 0x1001ec9;  /* U+1EC9 LATIN SMALL LETTER I WITH HOOK ABOVE */
pub const XKB_KEY_Ibelowdot                  :u32 = 0x1001eca;  /* U+1ECA LATIN CAPITAL LETTER I WITH DOT BELOW */
pub const XKB_KEY_ibelowdot                  :u32 = 0x1001ecb;  /* U+1ECB LATIN SMALL LETTER I WITH DOT BELOW */
pub const XKB_KEY_Obelowdot                  :u32 = 0x1001ecc;  /* U+1ECC LATIN CAPITAL LETTER O WITH DOT BELOW */
pub const XKB_KEY_obelowdot                  :u32 = 0x1001ecd;  /* U+1ECD LATIN SMALL LETTER O WITH DOT BELOW */
pub const XKB_KEY_Ohook                      :u32 = 0x1001ece;  /* U+1ECE LATIN CAPITAL LETTER O WITH HOOK ABOVE */
pub const XKB_KEY_ohook                      :u32 = 0x1001ecf;  /* U+1ECF LATIN SMALL LETTER O WITH HOOK ABOVE */
pub const XKB_KEY_Ocircumflexacute           :u32 = 0x1001ed0;  /* U+1ED0 LATIN CAPITAL LETTER O WITH CIRCUMFLEX AND ACUTE */
pub const XKB_KEY_ocircumflexacute           :u32 = 0x1001ed1;  /* U+1ED1 LATIN SMALL LETTER O WITH CIRCUMFLEX AND ACUTE */
pub const XKB_KEY_Ocircumflexgrave           :u32 = 0x1001ed2;  /* U+1ED2 LATIN CAPITAL LETTER O WITH CIRCUMFLEX AND GRAVE */
pub const XKB_KEY_ocircumflexgrave           :u32 = 0x1001ed3;  /* U+1ED3 LATIN SMALL LETTER O WITH CIRCUMFLEX AND GRAVE */
pub const XKB_KEY_Ocircumflexhook            :u32 = 0x1001ed4;  /* U+1ED4 LATIN CAPITAL LETTER O WITH CIRCUMFLEX AND HOOK ABOVE */
pub const XKB_KEY_ocircumflexhook            :u32 = 0x1001ed5;  /* U+1ED5 LATIN SMALL LETTER O WITH CIRCUMFLEX AND HOOK ABOVE */
pub const XKB_KEY_Ocircumflextilde           :u32 = 0x1001ed6;  /* U+1ED6 LATIN CAPITAL LETTER O WITH CIRCUMFLEX AND TILDE */
pub const XKB_KEY_ocircumflextilde           :u32 = 0x1001ed7;  /* U+1ED7 LATIN SMALL LETTER O WITH CIRCUMFLEX AND TILDE */
pub const XKB_KEY_Ocircumflexbelowdot        :u32 = 0x1001ed8;  /* U+1ED8 LATIN CAPITAL LETTER O WITH CIRCUMFLEX AND DOT BELOW */
pub const XKB_KEY_ocircumflexbelowdot        :u32 = 0x1001ed9;  /* U+1ED9 LATIN SMALL LETTER O WITH CIRCUMFLEX AND DOT BELOW */
pub const XKB_KEY_Ohornacute                 :u32 = 0x1001eda;  /* U+1EDA LATIN CAPITAL LETTER O WITH HORN AND ACUTE */
pub const XKB_KEY_ohornacute                 :u32 = 0x1001edb;  /* U+1EDB LATIN SMALL LETTER O WITH HORN AND ACUTE */
pub const XKB_KEY_Ohorngrave                 :u32 = 0x1001edc;  /* U+1EDC LATIN CAPITAL LETTER O WITH HORN AND GRAVE */
pub const XKB_KEY_ohorngrave                 :u32 = 0x1001edd;  /* U+1EDD LATIN SMALL LETTER O WITH HORN AND GRAVE */
pub const XKB_KEY_Ohornhook                  :u32 = 0x1001ede;  /* U+1EDE LATIN CAPITAL LETTER O WITH HORN AND HOOK ABOVE */
pub const XKB_KEY_ohornhook                  :u32 = 0x1001edf;  /* U+1EDF LATIN SMALL LETTER O WITH HORN AND HOOK ABOVE */
pub const XKB_KEY_Ohorntilde                 :u32 = 0x1001ee0;  /* U+1EE0 LATIN CAPITAL LETTER O WITH HORN AND TILDE */
pub const XKB_KEY_ohorntilde                 :u32 = 0x1001ee1;  /* U+1EE1 LATIN SMALL LETTER O WITH HORN AND TILDE */
pub const XKB_KEY_Ohornbelowdot              :u32 = 0x1001ee2;  /* U+1EE2 LATIN CAPITAL LETTER O WITH HORN AND DOT BELOW */
pub const XKB_KEY_ohornbelowdot              :u32 = 0x1001ee3;  /* U+1EE3 LATIN SMALL LETTER O WITH HORN AND DOT BELOW */
pub const XKB_KEY_Ubelowdot                  :u32 = 0x1001ee4;  /* U+1EE4 LATIN CAPITAL LETTER U WITH DOT BELOW */
pub const XKB_KEY_ubelowdot                  :u32 = 0x1001ee5;  /* U+1EE5 LATIN SMALL LETTER U WITH DOT BELOW */
pub const XKB_KEY_Uhook                      :u32 = 0x1001ee6;  /* U+1EE6 LATIN CAPITAL LETTER U WITH HOOK ABOVE */
pub const XKB_KEY_uhook                      :u32 = 0x1001ee7;  /* U+1EE7 LATIN SMALL LETTER U WITH HOOK ABOVE */
pub const XKB_KEY_Uhornacute                 :u32 = 0x1001ee8;  /* U+1EE8 LATIN CAPITAL LETTER U WITH HORN AND ACUTE */
pub const XKB_KEY_uhornacute                 :u32 = 0x1001ee9;  /* U+1EE9 LATIN SMALL LETTER U WITH HORN AND ACUTE */
pub const XKB_KEY_Uhorngrave                 :u32 = 0x1001eea;  /* U+1EEA LATIN CAPITAL LETTER U WITH HORN AND GRAVE */
pub const XKB_KEY_uhorngrave                 :u32 = 0x1001eeb;  /* U+1EEB LATIN SMALL LETTER U WITH HORN AND GRAVE */
pub const XKB_KEY_Uhornhook                  :u32 = 0x1001eec;  /* U+1EEC LATIN CAPITAL LETTER U WITH HORN AND HOOK ABOVE */
pub const XKB_KEY_uhornhook                  :u32 = 0x1001eed;  /* U+1EED LATIN SMALL LETTER U WITH HORN AND HOOK ABOVE */
pub const XKB_KEY_Uhorntilde                 :u32 = 0x1001eee;  /* U+1EEE LATIN CAPITAL LETTER U WITH HORN AND TILDE */
pub const XKB_KEY_uhorntilde                 :u32 = 0x1001eef;  /* U+1EEF LATIN SMALL LETTER U WITH HORN AND TILDE */
pub const XKB_KEY_Uhornbelowdot              :u32 = 0x1001ef0;  /* U+1EF0 LATIN CAPITAL LETTER U WITH HORN AND DOT BELOW */
pub const XKB_KEY_uhornbelowdot              :u32 = 0x1001ef1;  /* U+1EF1 LATIN SMALL LETTER U WITH HORN AND DOT BELOW */
pub const XKB_KEY_Ybelowdot                  :u32 = 0x1001ef4;  /* U+1EF4 LATIN CAPITAL LETTER Y WITH DOT BELOW */
pub const XKB_KEY_ybelowdot                  :u32 = 0x1001ef5;  /* U+1EF5 LATIN SMALL LETTER Y WITH DOT BELOW */
pub const XKB_KEY_Yhook                      :u32 = 0x1001ef6;  /* U+1EF6 LATIN CAPITAL LETTER Y WITH HOOK ABOVE */
pub const XKB_KEY_yhook                      :u32 = 0x1001ef7;  /* U+1EF7 LATIN SMALL LETTER Y WITH HOOK ABOVE */
pub const XKB_KEY_Ytilde                     :u32 = 0x1001ef8;  /* U+1EF8 LATIN CAPITAL LETTER Y WITH TILDE */
pub const XKB_KEY_ytilde                     :u32 = 0x1001ef9;  /* U+1EF9 LATIN SMALL LETTER Y WITH TILDE */
pub const XKB_KEY_Ohorn                      :u32 = 0x10001a0;  /* U+01A0 LATIN CAPITAL LETTER O WITH HORN */
pub const XKB_KEY_ohorn                      :u32 = 0x10001a1;  /* U+01A1 LATIN SMALL LETTER O WITH HORN */
pub const XKB_KEY_Uhorn                      :u32 = 0x10001af;  /* U+01AF LATIN CAPITAL LETTER U WITH HORN */
pub const XKB_KEY_uhorn                      :u32 = 0x10001b0;  /* U+01B0 LATIN SMALL LETTER U WITH HORN */


pub const XKB_KEY_EcuSign                    :u32 = 0x10020a0;  /* U+20A0 EURO-CURRENCY SIGN */
pub const XKB_KEY_ColonSign                  :u32 = 0x10020a1;  /* U+20A1 COLON SIGN */
pub const XKB_KEY_CruzeiroSign               :u32 = 0x10020a2;  /* U+20A2 CRUZEIRO SIGN */
pub const XKB_KEY_FFrancSign                 :u32 = 0x10020a3;  /* U+20A3 FRENCH FRANC SIGN */
pub const XKB_KEY_LiraSign                   :u32 = 0x10020a4;  /* U+20A4 LIRA SIGN */
pub const XKB_KEY_MillSign                   :u32 = 0x10020a5;  /* U+20A5 MILL SIGN */
pub const XKB_KEY_NairaSign                  :u32 = 0x10020a6;  /* U+20A6 NAIRA SIGN */
pub const XKB_KEY_PesetaSign                 :u32 = 0x10020a7;  /* U+20A7 PESETA SIGN */
pub const XKB_KEY_RupeeSign                  :u32 = 0x10020a8;  /* U+20A8 RUPEE SIGN */
pub const XKB_KEY_WonSign                    :u32 = 0x10020a9;  /* U+20A9 WON SIGN */
pub const XKB_KEY_NewSheqelSign              :u32 = 0x10020aa;  /* U+20AA NEW SHEQEL SIGN */
pub const XKB_KEY_DongSign                   :u32 = 0x10020ab;  /* U+20AB DONG SIGN */
pub const XKB_KEY_EuroSign                      :u32 = 0x20ac;  /* U+20AC EURO SIGN */

/* one, two and three are defined above. */
pub const XKB_KEY_zerosuperior               :u32 = 0x1002070;  /* U+2070 SUPERSCRIPT ZERO */
pub const XKB_KEY_foursuperior               :u32 = 0x1002074;  /* U+2074 SUPERSCRIPT FOUR */
pub const XKB_KEY_fivesuperior               :u32 = 0x1002075;  /* U+2075 SUPERSCRIPT FIVE */
pub const XKB_KEY_sixsuperior                :u32 = 0x1002076;  /* U+2076 SUPERSCRIPT SIX */
pub const XKB_KEY_sevensuperior              :u32 = 0x1002077;  /* U+2077 SUPERSCRIPT SEVEN */
pub const XKB_KEY_eightsuperior              :u32 = 0x1002078;  /* U+2078 SUPERSCRIPT EIGHT */
pub const XKB_KEY_ninesuperior               :u32 = 0x1002079;  /* U+2079 SUPERSCRIPT NINE */
pub const XKB_KEY_zerosubscript              :u32 = 0x1002080;  /* U+2080 SUBSCRIPT ZERO */
pub const XKB_KEY_onesubscript               :u32 = 0x1002081;  /* U+2081 SUBSCRIPT ONE */
pub const XKB_KEY_twosubscript               :u32 = 0x1002082;  /* U+2082 SUBSCRIPT TWO */
pub const XKB_KEY_threesubscript             :u32 = 0x1002083;  /* U+2083 SUBSCRIPT THREE */
pub const XKB_KEY_foursubscript              :u32 = 0x1002084;  /* U+2084 SUBSCRIPT FOUR */
pub const XKB_KEY_fivesubscript              :u32 = 0x1002085;  /* U+2085 SUBSCRIPT FIVE */
pub const XKB_KEY_sixsubscript               :u32 = 0x1002086;  /* U+2086 SUBSCRIPT SIX */
pub const XKB_KEY_sevensubscript             :u32 = 0x1002087;  /* U+2087 SUBSCRIPT SEVEN */
pub const XKB_KEY_eightsubscript             :u32 = 0x1002088;  /* U+2088 SUBSCRIPT EIGHT */
pub const XKB_KEY_ninesubscript              :u32 = 0x1002089;  /* U+2089 SUBSCRIPT NINE */
pub const XKB_KEY_partdifferential           :u32 = 0x1002202;  /* U+2202 PARTIAL DIFFERENTIAL */
pub const XKB_KEY_emptyset                   :u32 = 0x1002205;  /* U+2205 NULL SET */
pub const XKB_KEY_elementof                  :u32 = 0x1002208;  /* U+2208 ELEMENT OF */
pub const XKB_KEY_notelementof               :u32 = 0x1002209;  /* U+2209 NOT AN ELEMENT OF */
pub const XKB_KEY_containsas                 :u32 = 0x100220B;  /* U+220B CONTAINS AS MEMBER */
pub const XKB_KEY_squareroot                 :u32 = 0x100221A;  /* U+221A SQUARE ROOT */
pub const XKB_KEY_cuberoot                   :u32 = 0x100221B;  /* U+221B CUBE ROOT */
pub const XKB_KEY_fourthroot                 :u32 = 0x100221C;  /* U+221C FOURTH ROOT */
pub const XKB_KEY_dintegral                  :u32 = 0x100222C;  /* U+222C DOUBLE INTEGRAL */
pub const XKB_KEY_tintegral                  :u32 = 0x100222D;  /* U+222D TRIPLE INTEGRAL */
pub const XKB_KEY_because                    :u32 = 0x1002235;  /* U+2235 BECAUSE */
pub const XKB_KEY_approxeq                   :u32 = 0x1002248;  /* U+2245 ALMOST EQUAL TO */
pub const XKB_KEY_notapproxeq                :u32 = 0x1002247;  /* U+2247 NOT ALMOST EQUAL TO */
pub const XKB_KEY_notidentical               :u32 = 0x1002262;  /* U+2262 NOT IDENTICAL TO */
pub const XKB_KEY_stricteq                   :u32 = 0x1002263;  /* U+2263 STRICTLY EQUIVALENT TO */

pub const XKB_KEY_braille_dot_1                 :u32 = 0xfff1;
pub const XKB_KEY_braille_dot_2                 :u32 = 0xfff2;
pub const XKB_KEY_braille_dot_3                 :u32 = 0xfff3;
pub const XKB_KEY_braille_dot_4                 :u32 = 0xfff4;
pub const XKB_KEY_braille_dot_5                 :u32 = 0xfff5;
pub const XKB_KEY_braille_dot_6                 :u32 = 0xfff6;
pub const XKB_KEY_braille_dot_7                 :u32 = 0xfff7;
pub const XKB_KEY_braille_dot_8                 :u32 = 0xfff8;
pub const XKB_KEY_braille_dot_9                 :u32 = 0xfff9;
pub const XKB_KEY_braille_dot_10                :u32 = 0xfffa;
pub const XKB_KEY_braille_blank              :u32 = 0x1002800;  /* U+2800 BRAILLE PATTERN BLANK */
pub const XKB_KEY_braille_dots_1             :u32 = 0x1002801;  /* U+2801 BRAILLE PATTERN DOTS-1 */
pub const XKB_KEY_braille_dots_2             :u32 = 0x1002802;  /* U+2802 BRAILLE PATTERN DOTS-2 */
pub const XKB_KEY_braille_dots_12            :u32 = 0x1002803;  /* U+2803 BRAILLE PATTERN DOTS-12 */
pub const XKB_KEY_braille_dots_3             :u32 = 0x1002804;  /* U+2804 BRAILLE PATTERN DOTS-3 */
pub const XKB_KEY_braille_dots_13            :u32 = 0x1002805;  /* U+2805 BRAILLE PATTERN DOTS-13 */
pub const XKB_KEY_braille_dots_23            :u32 = 0x1002806;  /* U+2806 BRAILLE PATTERN DOTS-23 */
pub const XKB_KEY_braille_dots_123           :u32 = 0x1002807;  /* U+2807 BRAILLE PATTERN DOTS-123 */
pub const XKB_KEY_braille_dots_4             :u32 = 0x1002808;  /* U+2808 BRAILLE PATTERN DOTS-4 */
pub const XKB_KEY_braille_dots_14            :u32 = 0x1002809;  /* U+2809 BRAILLE PATTERN DOTS-14 */
pub const XKB_KEY_braille_dots_24            :u32 = 0x100280a;  /* U+280a BRAILLE PATTERN DOTS-24 */
pub const XKB_KEY_braille_dots_124           :u32 = 0x100280b;  /* U+280b BRAILLE PATTERN DOTS-124 */
pub const XKB_KEY_braille_dots_34            :u32 = 0x100280c;  /* U+280c BRAILLE PATTERN DOTS-34 */
pub const XKB_KEY_braille_dots_134           :u32 = 0x100280d;  /* U+280d BRAILLE PATTERN DOTS-134 */
pub const XKB_KEY_braille_dots_234           :u32 = 0x100280e;  /* U+280e BRAILLE PATTERN DOTS-234 */
pub const XKB_KEY_braille_dots_1234          :u32 = 0x100280f;  /* U+280f BRAILLE PATTERN DOTS-1234 */
pub const XKB_KEY_braille_dots_5             :u32 = 0x1002810;  /* U+2810 BRAILLE PATTERN DOTS-5 */
pub const XKB_KEY_braille_dots_15            :u32 = 0x1002811;  /* U+2811 BRAILLE PATTERN DOTS-15 */
pub const XKB_KEY_braille_dots_25            :u32 = 0x1002812;  /* U+2812 BRAILLE PATTERN DOTS-25 */
pub const XKB_KEY_braille_dots_125           :u32 = 0x1002813;  /* U+2813 BRAILLE PATTERN DOTS-125 */
pub const XKB_KEY_braille_dots_35            :u32 = 0x1002814;  /* U+2814 BRAILLE PATTERN DOTS-35 */
pub const XKB_KEY_braille_dots_135           :u32 = 0x1002815;  /* U+2815 BRAILLE PATTERN DOTS-135 */
pub const XKB_KEY_braille_dots_235           :u32 = 0x1002816;  /* U+2816 BRAILLE PATTERN DOTS-235 */
pub const XKB_KEY_braille_dots_1235          :u32 = 0x1002817;  /* U+2817 BRAILLE PATTERN DOTS-1235 */
pub const XKB_KEY_braille_dots_45            :u32 = 0x1002818;  /* U+2818 BRAILLE PATTERN DOTS-45 */
pub const XKB_KEY_braille_dots_145           :u32 = 0x1002819;  /* U+2819 BRAILLE PATTERN DOTS-145 */
pub const XKB_KEY_braille_dots_245           :u32 = 0x100281a;  /* U+281a BRAILLE PATTERN DOTS-245 */
pub const XKB_KEY_braille_dots_1245          :u32 = 0x100281b;  /* U+281b BRAILLE PATTERN DOTS-1245 */
pub const XKB_KEY_braille_dots_345           :u32 = 0x100281c;  /* U+281c BRAILLE PATTERN DOTS-345 */
pub const XKB_KEY_braille_dots_1345          :u32 = 0x100281d;  /* U+281d BRAILLE PATTERN DOTS-1345 */
pub const XKB_KEY_braille_dots_2345          :u32 = 0x100281e;  /* U+281e BRAILLE PATTERN DOTS-2345 */
pub const XKB_KEY_braille_dots_12345         :u32 = 0x100281f;  /* U+281f BRAILLE PATTERN DOTS-12345 */
pub const XKB_KEY_braille_dots_6             :u32 = 0x1002820;  /* U+2820 BRAILLE PATTERN DOTS-6 */
pub const XKB_KEY_braille_dots_16            :u32 = 0x1002821;  /* U+2821 BRAILLE PATTERN DOTS-16 */
pub const XKB_KEY_braille_dots_26            :u32 = 0x1002822;  /* U+2822 BRAILLE PATTERN DOTS-26 */
pub const XKB_KEY_braille_dots_126           :u32 = 0x1002823;  /* U+2823 BRAILLE PATTERN DOTS-126 */
pub const XKB_KEY_braille_dots_36            :u32 = 0x1002824;  /* U+2824 BRAILLE PATTERN DOTS-36 */
pub const XKB_KEY_braille_dots_136           :u32 = 0x1002825;  /* U+2825 BRAILLE PATTERN DOTS-136 */
pub const XKB_KEY_braille_dots_236           :u32 = 0x1002826;  /* U+2826 BRAILLE PATTERN DOTS-236 */
pub const XKB_KEY_braille_dots_1236          :u32 = 0x1002827;  /* U+2827 BRAILLE PATTERN DOTS-1236 */
pub const XKB_KEY_braille_dots_46            :u32 = 0x1002828;  /* U+2828 BRAILLE PATTERN DOTS-46 */
pub const XKB_KEY_braille_dots_146           :u32 = 0x1002829;  /* U+2829 BRAILLE PATTERN DOTS-146 */
pub const XKB_KEY_braille_dots_246           :u32 = 0x100282a;  /* U+282a BRAILLE PATTERN DOTS-246 */
pub const XKB_KEY_braille_dots_1246          :u32 = 0x100282b;  /* U+282b BRAILLE PATTERN DOTS-1246 */
pub const XKB_KEY_braille_dots_346           :u32 = 0x100282c;  /* U+282c BRAILLE PATTERN DOTS-346 */
pub const XKB_KEY_braille_dots_1346          :u32 = 0x100282d;  /* U+282d BRAILLE PATTERN DOTS-1346 */
pub const XKB_KEY_braille_dots_2346          :u32 = 0x100282e;  /* U+282e BRAILLE PATTERN DOTS-2346 */
pub const XKB_KEY_braille_dots_12346         :u32 = 0x100282f;  /* U+282f BRAILLE PATTERN DOTS-12346 */
pub const XKB_KEY_braille_dots_56            :u32 = 0x1002830;  /* U+2830 BRAILLE PATTERN DOTS-56 */
pub const XKB_KEY_braille_dots_156           :u32 = 0x1002831;  /* U+2831 BRAILLE PATTERN DOTS-156 */
pub const XKB_KEY_braille_dots_256           :u32 = 0x1002832;  /* U+2832 BRAILLE PATTERN DOTS-256 */
pub const XKB_KEY_braille_dots_1256          :u32 = 0x1002833;  /* U+2833 BRAILLE PATTERN DOTS-1256 */
pub const XKB_KEY_braille_dots_356           :u32 = 0x1002834;  /* U+2834 BRAILLE PATTERN DOTS-356 */
pub const XKB_KEY_braille_dots_1356          :u32 = 0x1002835;  /* U+2835 BRAILLE PATTERN DOTS-1356 */
pub const XKB_KEY_braille_dots_2356          :u32 = 0x1002836;  /* U+2836 BRAILLE PATTERN DOTS-2356 */
pub const XKB_KEY_braille_dots_12356         :u32 = 0x1002837;  /* U+2837 BRAILLE PATTERN DOTS-12356 */
pub const XKB_KEY_braille_dots_456           :u32 = 0x1002838;  /* U+2838 BRAILLE PATTERN DOTS-456 */
pub const XKB_KEY_braille_dots_1456          :u32 = 0x1002839;  /* U+2839 BRAILLE PATTERN DOTS-1456 */
pub const XKB_KEY_braille_dots_2456          :u32 = 0x100283a;  /* U+283a BRAILLE PATTERN DOTS-2456 */
pub const XKB_KEY_braille_dots_12456         :u32 = 0x100283b;  /* U+283b BRAILLE PATTERN DOTS-12456 */
pub const XKB_KEY_braille_dots_3456          :u32 = 0x100283c;  /* U+283c BRAILLE PATTERN DOTS-3456 */
pub const XKB_KEY_braille_dots_13456         :u32 = 0x100283d;  /* U+283d BRAILLE PATTERN DOTS-13456 */
pub const XKB_KEY_braille_dots_23456         :u32 = 0x100283e;  /* U+283e BRAILLE PATTERN DOTS-23456 */
pub const XKB_KEY_braille_dots_123456        :u32 = 0x100283f;  /* U+283f BRAILLE PATTERN DOTS-123456 */
pub const XKB_KEY_braille_dots_7             :u32 = 0x1002840;  /* U+2840 BRAILLE PATTERN DOTS-7 */
pub const XKB_KEY_braille_dots_17            :u32 = 0x1002841;  /* U+2841 BRAILLE PATTERN DOTS-17 */
pub const XKB_KEY_braille_dots_27            :u32 = 0x1002842;  /* U+2842 BRAILLE PATTERN DOTS-27 */
pub const XKB_KEY_braille_dots_127           :u32 = 0x1002843;  /* U+2843 BRAILLE PATTERN DOTS-127 */
pub const XKB_KEY_braille_dots_37            :u32 = 0x1002844;  /* U+2844 BRAILLE PATTERN DOTS-37 */
pub const XKB_KEY_braille_dots_137           :u32 = 0x1002845;  /* U+2845 BRAILLE PATTERN DOTS-137 */
pub const XKB_KEY_braille_dots_237           :u32 = 0x1002846;  /* U+2846 BRAILLE PATTERN DOTS-237 */
pub const XKB_KEY_braille_dots_1237          :u32 = 0x1002847;  /* U+2847 BRAILLE PATTERN DOTS-1237 */
pub const XKB_KEY_braille_dots_47            :u32 = 0x1002848;  /* U+2848 BRAILLE PATTERN DOTS-47 */
pub const XKB_KEY_braille_dots_147           :u32 = 0x1002849;  /* U+2849 BRAILLE PATTERN DOTS-147 */
pub const XKB_KEY_braille_dots_247           :u32 = 0x100284a;  /* U+284a BRAILLE PATTERN DOTS-247 */
pub const XKB_KEY_braille_dots_1247          :u32 = 0x100284b;  /* U+284b BRAILLE PATTERN DOTS-1247 */
pub const XKB_KEY_braille_dots_347           :u32 = 0x100284c;  /* U+284c BRAILLE PATTERN DOTS-347 */
pub const XKB_KEY_braille_dots_1347          :u32 = 0x100284d;  /* U+284d BRAILLE PATTERN DOTS-1347 */
pub const XKB_KEY_braille_dots_2347          :u32 = 0x100284e;  /* U+284e BRAILLE PATTERN DOTS-2347 */
pub const XKB_KEY_braille_dots_12347         :u32 = 0x100284f;  /* U+284f BRAILLE PATTERN DOTS-12347 */
pub const XKB_KEY_braille_dots_57            :u32 = 0x1002850;  /* U+2850 BRAILLE PATTERN DOTS-57 */
pub const XKB_KEY_braille_dots_157           :u32 = 0x1002851;  /* U+2851 BRAILLE PATTERN DOTS-157 */
pub const XKB_KEY_braille_dots_257           :u32 = 0x1002852;  /* U+2852 BRAILLE PATTERN DOTS-257 */
pub const XKB_KEY_braille_dots_1257          :u32 = 0x1002853;  /* U+2853 BRAILLE PATTERN DOTS-1257 */
pub const XKB_KEY_braille_dots_357           :u32 = 0x1002854;  /* U+2854 BRAILLE PATTERN DOTS-357 */
pub const XKB_KEY_braille_dots_1357          :u32 = 0x1002855;  /* U+2855 BRAILLE PATTERN DOTS-1357 */
pub const XKB_KEY_braille_dots_2357          :u32 = 0x1002856;  /* U+2856 BRAILLE PATTERN DOTS-2357 */
pub const XKB_KEY_braille_dots_12357         :u32 = 0x1002857;  /* U+2857 BRAILLE PATTERN DOTS-12357 */
pub const XKB_KEY_braille_dots_457           :u32 = 0x1002858;  /* U+2858 BRAILLE PATTERN DOTS-457 */
pub const XKB_KEY_braille_dots_1457          :u32 = 0x1002859;  /* U+2859 BRAILLE PATTERN DOTS-1457 */
pub const XKB_KEY_braille_dots_2457          :u32 = 0x100285a;  /* U+285a BRAILLE PATTERN DOTS-2457 */
pub const XKB_KEY_braille_dots_12457         :u32 = 0x100285b;  /* U+285b BRAILLE PATTERN DOTS-12457 */
pub const XKB_KEY_braille_dots_3457          :u32 = 0x100285c;  /* U+285c BRAILLE PATTERN DOTS-3457 */
pub const XKB_KEY_braille_dots_13457         :u32 = 0x100285d;  /* U+285d BRAILLE PATTERN DOTS-13457 */
pub const XKB_KEY_braille_dots_23457         :u32 = 0x100285e;  /* U+285e BRAILLE PATTERN DOTS-23457 */
pub const XKB_KEY_braille_dots_123457        :u32 = 0x100285f;  /* U+285f BRAILLE PATTERN DOTS-123457 */
pub const XKB_KEY_braille_dots_67            :u32 = 0x1002860;  /* U+2860 BRAILLE PATTERN DOTS-67 */
pub const XKB_KEY_braille_dots_167           :u32 = 0x1002861;  /* U+2861 BRAILLE PATTERN DOTS-167 */
pub const XKB_KEY_braille_dots_267           :u32 = 0x1002862;  /* U+2862 BRAILLE PATTERN DOTS-267 */
pub const XKB_KEY_braille_dots_1267          :u32 = 0x1002863;  /* U+2863 BRAILLE PATTERN DOTS-1267 */
pub const XKB_KEY_braille_dots_367           :u32 = 0x1002864;  /* U+2864 BRAILLE PATTERN DOTS-367 */
pub const XKB_KEY_braille_dots_1367          :u32 = 0x1002865;  /* U+2865 BRAILLE PATTERN DOTS-1367 */
pub const XKB_KEY_braille_dots_2367          :u32 = 0x1002866;  /* U+2866 BRAILLE PATTERN DOTS-2367 */
pub const XKB_KEY_braille_dots_12367         :u32 = 0x1002867;  /* U+2867 BRAILLE PATTERN DOTS-12367 */
pub const XKB_KEY_braille_dots_467           :u32 = 0x1002868;  /* U+2868 BRAILLE PATTERN DOTS-467 */
pub const XKB_KEY_braille_dots_1467          :u32 = 0x1002869;  /* U+2869 BRAILLE PATTERN DOTS-1467 */
pub const XKB_KEY_braille_dots_2467          :u32 = 0x100286a;  /* U+286a BRAILLE PATTERN DOTS-2467 */
pub const XKB_KEY_braille_dots_12467         :u32 = 0x100286b;  /* U+286b BRAILLE PATTERN DOTS-12467 */
pub const XKB_KEY_braille_dots_3467          :u32 = 0x100286c;  /* U+286c BRAILLE PATTERN DOTS-3467 */
pub const XKB_KEY_braille_dots_13467         :u32 = 0x100286d;  /* U+286d BRAILLE PATTERN DOTS-13467 */
pub const XKB_KEY_braille_dots_23467         :u32 = 0x100286e;  /* U+286e BRAILLE PATTERN DOTS-23467 */
pub const XKB_KEY_braille_dots_123467        :u32 = 0x100286f;  /* U+286f BRAILLE PATTERN DOTS-123467 */
pub const XKB_KEY_braille_dots_567           :u32 = 0x1002870;  /* U+2870 BRAILLE PATTERN DOTS-567 */
pub const XKB_KEY_braille_dots_1567          :u32 = 0x1002871;  /* U+2871 BRAILLE PATTERN DOTS-1567 */
pub const XKB_KEY_braille_dots_2567          :u32 = 0x1002872;  /* U+2872 BRAILLE PATTERN DOTS-2567 */
pub const XKB_KEY_braille_dots_12567         :u32 = 0x1002873;  /* U+2873 BRAILLE PATTERN DOTS-12567 */
pub const XKB_KEY_braille_dots_3567          :u32 = 0x1002874;  /* U+2874 BRAILLE PATTERN DOTS-3567 */
pub const XKB_KEY_braille_dots_13567         :u32 = 0x1002875;  /* U+2875 BRAILLE PATTERN DOTS-13567 */
pub const XKB_KEY_braille_dots_23567         :u32 = 0x1002876;  /* U+2876 BRAILLE PATTERN DOTS-23567 */
pub const XKB_KEY_braille_dots_123567        :u32 = 0x1002877;  /* U+2877 BRAILLE PATTERN DOTS-123567 */
pub const XKB_KEY_braille_dots_4567          :u32 = 0x1002878;  /* U+2878 BRAILLE PATTERN DOTS-4567 */
pub const XKB_KEY_braille_dots_14567         :u32 = 0x1002879;  /* U+2879 BRAILLE PATTERN DOTS-14567 */
pub const XKB_KEY_braille_dots_24567         :u32 = 0x100287a;  /* U+287a BRAILLE PATTERN DOTS-24567 */
pub const XKB_KEY_braille_dots_124567        :u32 = 0x100287b;  /* U+287b BRAILLE PATTERN DOTS-124567 */
pub const XKB_KEY_braille_dots_34567         :u32 = 0x100287c;  /* U+287c BRAILLE PATTERN DOTS-34567 */
pub const XKB_KEY_braille_dots_134567        :u32 = 0x100287d;  /* U+287d BRAILLE PATTERN DOTS-134567 */
pub const XKB_KEY_braille_dots_234567        :u32 = 0x100287e;  /* U+287e BRAILLE PATTERN DOTS-234567 */
pub const XKB_KEY_braille_dots_1234567       :u32 = 0x100287f;  /* U+287f BRAILLE PATTERN DOTS-1234567 */
pub const XKB_KEY_braille_dots_8             :u32 = 0x1002880;  /* U+2880 BRAILLE PATTERN DOTS-8 */
pub const XKB_KEY_braille_dots_18            :u32 = 0x1002881;  /* U+2881 BRAILLE PATTERN DOTS-18 */
pub const XKB_KEY_braille_dots_28            :u32 = 0x1002882;  /* U+2882 BRAILLE PATTERN DOTS-28 */
pub const XKB_KEY_braille_dots_128           :u32 = 0x1002883;  /* U+2883 BRAILLE PATTERN DOTS-128 */
pub const XKB_KEY_braille_dots_38            :u32 = 0x1002884;  /* U+2884 BRAILLE PATTERN DOTS-38 */
pub const XKB_KEY_braille_dots_138           :u32 = 0x1002885;  /* U+2885 BRAILLE PATTERN DOTS-138 */
pub const XKB_KEY_braille_dots_238           :u32 = 0x1002886;  /* U+2886 BRAILLE PATTERN DOTS-238 */
pub const XKB_KEY_braille_dots_1238          :u32 = 0x1002887;  /* U+2887 BRAILLE PATTERN DOTS-1238 */
pub const XKB_KEY_braille_dots_48            :u32 = 0x1002888;  /* U+2888 BRAILLE PATTERN DOTS-48 */
pub const XKB_KEY_braille_dots_148           :u32 = 0x1002889;  /* U+2889 BRAILLE PATTERN DOTS-148 */
pub const XKB_KEY_braille_dots_248           :u32 = 0x100288a;  /* U+288a BRAILLE PATTERN DOTS-248 */
pub const XKB_KEY_braille_dots_1248          :u32 = 0x100288b;  /* U+288b BRAILLE PATTERN DOTS-1248 */
pub const XKB_KEY_braille_dots_348           :u32 = 0x100288c;  /* U+288c BRAILLE PATTERN DOTS-348 */
pub const XKB_KEY_braille_dots_1348          :u32 = 0x100288d;  /* U+288d BRAILLE PATTERN DOTS-1348 */
pub const XKB_KEY_braille_dots_2348          :u32 = 0x100288e;  /* U+288e BRAILLE PATTERN DOTS-2348 */
pub const XKB_KEY_braille_dots_12348         :u32 = 0x100288f;  /* U+288f BRAILLE PATTERN DOTS-12348 */
pub const XKB_KEY_braille_dots_58            :u32 = 0x1002890;  /* U+2890 BRAILLE PATTERN DOTS-58 */
pub const XKB_KEY_braille_dots_158           :u32 = 0x1002891;  /* U+2891 BRAILLE PATTERN DOTS-158 */
pub const XKB_KEY_braille_dots_258           :u32 = 0x1002892;  /* U+2892 BRAILLE PATTERN DOTS-258 */
pub const XKB_KEY_braille_dots_1258          :u32 = 0x1002893;  /* U+2893 BRAILLE PATTERN DOTS-1258 */
pub const XKB_KEY_braille_dots_358           :u32 = 0x1002894;  /* U+2894 BRAILLE PATTERN DOTS-358 */
pub const XKB_KEY_braille_dots_1358          :u32 = 0x1002895;  /* U+2895 BRAILLE PATTERN DOTS-1358 */
pub const XKB_KEY_braille_dots_2358          :u32 = 0x1002896;  /* U+2896 BRAILLE PATTERN DOTS-2358 */
pub const XKB_KEY_braille_dots_12358         :u32 = 0x1002897;  /* U+2897 BRAILLE PATTERN DOTS-12358 */
pub const XKB_KEY_braille_dots_458           :u32 = 0x1002898;  /* U+2898 BRAILLE PATTERN DOTS-458 */
pub const XKB_KEY_braille_dots_1458          :u32 = 0x1002899;  /* U+2899 BRAILLE PATTERN DOTS-1458 */
pub const XKB_KEY_braille_dots_2458          :u32 = 0x100289a;  /* U+289a BRAILLE PATTERN DOTS-2458 */
pub const XKB_KEY_braille_dots_12458         :u32 = 0x100289b;  /* U+289b BRAILLE PATTERN DOTS-12458 */
pub const XKB_KEY_braille_dots_3458          :u32 = 0x100289c;  /* U+289c BRAILLE PATTERN DOTS-3458 */
pub const XKB_KEY_braille_dots_13458         :u32 = 0x100289d;  /* U+289d BRAILLE PATTERN DOTS-13458 */
pub const XKB_KEY_braille_dots_23458         :u32 = 0x100289e;  /* U+289e BRAILLE PATTERN DOTS-23458 */
pub const XKB_KEY_braille_dots_123458        :u32 = 0x100289f;  /* U+289f BRAILLE PATTERN DOTS-123458 */
pub const XKB_KEY_braille_dots_68            :u32 = 0x10028a0;  /* U+28a0 BRAILLE PATTERN DOTS-68 */
pub const XKB_KEY_braille_dots_168           :u32 = 0x10028a1;  /* U+28a1 BRAILLE PATTERN DOTS-168 */
pub const XKB_KEY_braille_dots_268           :u32 = 0x10028a2;  /* U+28a2 BRAILLE PATTERN DOTS-268 */
pub const XKB_KEY_braille_dots_1268          :u32 = 0x10028a3;  /* U+28a3 BRAILLE PATTERN DOTS-1268 */
pub const XKB_KEY_braille_dots_368           :u32 = 0x10028a4;  /* U+28a4 BRAILLE PATTERN DOTS-368 */
pub const XKB_KEY_braille_dots_1368          :u32 = 0x10028a5;  /* U+28a5 BRAILLE PATTERN DOTS-1368 */
pub const XKB_KEY_braille_dots_2368          :u32 = 0x10028a6;  /* U+28a6 BRAILLE PATTERN DOTS-2368 */
pub const XKB_KEY_braille_dots_12368         :u32 = 0x10028a7;  /* U+28a7 BRAILLE PATTERN DOTS-12368 */
pub const XKB_KEY_braille_dots_468           :u32 = 0x10028a8;  /* U+28a8 BRAILLE PATTERN DOTS-468 */
pub const XKB_KEY_braille_dots_1468          :u32 = 0x10028a9;  /* U+28a9 BRAILLE PATTERN DOTS-1468 */
pub const XKB_KEY_braille_dots_2468          :u32 = 0x10028aa;  /* U+28aa BRAILLE PATTERN DOTS-2468 */
pub const XKB_KEY_braille_dots_12468         :u32 = 0x10028ab;  /* U+28ab BRAILLE PATTERN DOTS-12468 */
pub const XKB_KEY_braille_dots_3468          :u32 = 0x10028ac;  /* U+28ac BRAILLE PATTERN DOTS-3468 */
pub const XKB_KEY_braille_dots_13468         :u32 = 0x10028ad;  /* U+28ad BRAILLE PATTERN DOTS-13468 */
pub const XKB_KEY_braille_dots_23468         :u32 = 0x10028ae;  /* U+28ae BRAILLE PATTERN DOTS-23468 */
pub const XKB_KEY_braille_dots_123468        :u32 = 0x10028af;  /* U+28af BRAILLE PATTERN DOTS-123468 */
pub const XKB_KEY_braille_dots_568           :u32 = 0x10028b0;  /* U+28b0 BRAILLE PATTERN DOTS-568 */
pub const XKB_KEY_braille_dots_1568          :u32 = 0x10028b1;  /* U+28b1 BRAILLE PATTERN DOTS-1568 */
pub const XKB_KEY_braille_dots_2568          :u32 = 0x10028b2;  /* U+28b2 BRAILLE PATTERN DOTS-2568 */
pub const XKB_KEY_braille_dots_12568         :u32 = 0x10028b3;  /* U+28b3 BRAILLE PATTERN DOTS-12568 */
pub const XKB_KEY_braille_dots_3568          :u32 = 0x10028b4;  /* U+28b4 BRAILLE PATTERN DOTS-3568 */
pub const XKB_KEY_braille_dots_13568         :u32 = 0x10028b5;  /* U+28b5 BRAILLE PATTERN DOTS-13568 */
pub const XKB_KEY_braille_dots_23568         :u32 = 0x10028b6;  /* U+28b6 BRAILLE PATTERN DOTS-23568 */
pub const XKB_KEY_braille_dots_123568        :u32 = 0x10028b7;  /* U+28b7 BRAILLE PATTERN DOTS-123568 */
pub const XKB_KEY_braille_dots_4568          :u32 = 0x10028b8;  /* U+28b8 BRAILLE PATTERN DOTS-4568 */
pub const XKB_KEY_braille_dots_14568         :u32 = 0x10028b9;  /* U+28b9 BRAILLE PATTERN DOTS-14568 */
pub const XKB_KEY_braille_dots_24568         :u32 = 0x10028ba;  /* U+28ba BRAILLE PATTERN DOTS-24568 */
pub const XKB_KEY_braille_dots_124568        :u32 = 0x10028bb;  /* U+28bb BRAILLE PATTERN DOTS-124568 */
pub const XKB_KEY_braille_dots_34568         :u32 = 0x10028bc;  /* U+28bc BRAILLE PATTERN DOTS-34568 */
pub const XKB_KEY_braille_dots_134568        :u32 = 0x10028bd;  /* U+28bd BRAILLE PATTERN DOTS-134568 */
pub const XKB_KEY_braille_dots_234568        :u32 = 0x10028be;  /* U+28be BRAILLE PATTERN DOTS-234568 */
pub const XKB_KEY_braille_dots_1234568       :u32 = 0x10028bf;  /* U+28bf BRAILLE PATTERN DOTS-1234568 */
pub const XKB_KEY_braille_dots_78            :u32 = 0x10028c0;  /* U+28c0 BRAILLE PATTERN DOTS-78 */
pub const XKB_KEY_braille_dots_178           :u32 = 0x10028c1;  /* U+28c1 BRAILLE PATTERN DOTS-178 */
pub const XKB_KEY_braille_dots_278           :u32 = 0x10028c2;  /* U+28c2 BRAILLE PATTERN DOTS-278 */
pub const XKB_KEY_braille_dots_1278          :u32 = 0x10028c3;  /* U+28c3 BRAILLE PATTERN DOTS-1278 */
pub const XKB_KEY_braille_dots_378           :u32 = 0x10028c4;  /* U+28c4 BRAILLE PATTERN DOTS-378 */
pub const XKB_KEY_braille_dots_1378          :u32 = 0x10028c5;  /* U+28c5 BRAILLE PATTERN DOTS-1378 */
pub const XKB_KEY_braille_dots_2378          :u32 = 0x10028c6;  /* U+28c6 BRAILLE PATTERN DOTS-2378 */
pub const XKB_KEY_braille_dots_12378         :u32 = 0x10028c7;  /* U+28c7 BRAILLE PATTERN DOTS-12378 */
pub const XKB_KEY_braille_dots_478           :u32 = 0x10028c8;  /* U+28c8 BRAILLE PATTERN DOTS-478 */
pub const XKB_KEY_braille_dots_1478          :u32 = 0x10028c9;  /* U+28c9 BRAILLE PATTERN DOTS-1478 */
pub const XKB_KEY_braille_dots_2478          :u32 = 0x10028ca;  /* U+28ca BRAILLE PATTERN DOTS-2478 */
pub const XKB_KEY_braille_dots_12478         :u32 = 0x10028cb;  /* U+28cb BRAILLE PATTERN DOTS-12478 */
pub const XKB_KEY_braille_dots_3478          :u32 = 0x10028cc;  /* U+28cc BRAILLE PATTERN DOTS-3478 */
pub const XKB_KEY_braille_dots_13478         :u32 = 0x10028cd;  /* U+28cd BRAILLE PATTERN DOTS-13478 */
pub const XKB_KEY_braille_dots_23478         :u32 = 0x10028ce;  /* U+28ce BRAILLE PATTERN DOTS-23478 */
pub const XKB_KEY_braille_dots_123478        :u32 = 0x10028cf;  /* U+28cf BRAILLE PATTERN DOTS-123478 */
pub const XKB_KEY_braille_dots_578           :u32 = 0x10028d0;  /* U+28d0 BRAILLE PATTERN DOTS-578 */
pub const XKB_KEY_braille_dots_1578          :u32 = 0x10028d1;  /* U+28d1 BRAILLE PATTERN DOTS-1578 */
pub const XKB_KEY_braille_dots_2578          :u32 = 0x10028d2;  /* U+28d2 BRAILLE PATTERN DOTS-2578 */
pub const XKB_KEY_braille_dots_12578         :u32 = 0x10028d3;  /* U+28d3 BRAILLE PATTERN DOTS-12578 */
pub const XKB_KEY_braille_dots_3578          :u32 = 0x10028d4;  /* U+28d4 BRAILLE PATTERN DOTS-3578 */
pub const XKB_KEY_braille_dots_13578         :u32 = 0x10028d5;  /* U+28d5 BRAILLE PATTERN DOTS-13578 */
pub const XKB_KEY_braille_dots_23578         :u32 = 0x10028d6;  /* U+28d6 BRAILLE PATTERN DOTS-23578 */
pub const XKB_KEY_braille_dots_123578        :u32 = 0x10028d7;  /* U+28d7 BRAILLE PATTERN DOTS-123578 */
pub const XKB_KEY_braille_dots_4578          :u32 = 0x10028d8;  /* U+28d8 BRAILLE PATTERN DOTS-4578 */
pub const XKB_KEY_braille_dots_14578         :u32 = 0x10028d9;  /* U+28d9 BRAILLE PATTERN DOTS-14578 */
pub const XKB_KEY_braille_dots_24578         :u32 = 0x10028da;  /* U+28da BRAILLE PATTERN DOTS-24578 */
pub const XKB_KEY_braille_dots_124578        :u32 = 0x10028db;  /* U+28db BRAILLE PATTERN DOTS-124578 */
pub const XKB_KEY_braille_dots_34578         :u32 = 0x10028dc;  /* U+28dc BRAILLE PATTERN DOTS-34578 */
pub const XKB_KEY_braille_dots_134578        :u32 = 0x10028dd;  /* U+28dd BRAILLE PATTERN DOTS-134578 */
pub const XKB_KEY_braille_dots_234578        :u32 = 0x10028de;  /* U+28de BRAILLE PATTERN DOTS-234578 */
pub const XKB_KEY_braille_dots_1234578       :u32 = 0x10028df;  /* U+28df BRAILLE PATTERN DOTS-1234578 */
pub const XKB_KEY_braille_dots_678           :u32 = 0x10028e0;  /* U+28e0 BRAILLE PATTERN DOTS-678 */
pub const XKB_KEY_braille_dots_1678          :u32 = 0x10028e1;  /* U+28e1 BRAILLE PATTERN DOTS-1678 */
pub const XKB_KEY_braille_dots_2678          :u32 = 0x10028e2;  /* U+28e2 BRAILLE PATTERN DOTS-2678 */
pub const XKB_KEY_braille_dots_12678         :u32 = 0x10028e3;  /* U+28e3 BRAILLE PATTERN DOTS-12678 */
pub const XKB_KEY_braille_dots_3678          :u32 = 0x10028e4;  /* U+28e4 BRAILLE PATTERN DOTS-3678 */
pub const XKB_KEY_braille_dots_13678         :u32 = 0x10028e5;  /* U+28e5 BRAILLE PATTERN DOTS-13678 */
pub const XKB_KEY_braille_dots_23678         :u32 = 0x10028e6;  /* U+28e6 BRAILLE PATTERN DOTS-23678 */
pub const XKB_KEY_braille_dots_123678        :u32 = 0x10028e7;  /* U+28e7 BRAILLE PATTERN DOTS-123678 */
pub const XKB_KEY_braille_dots_4678          :u32 = 0x10028e8;  /* U+28e8 BRAILLE PATTERN DOTS-4678 */
pub const XKB_KEY_braille_dots_14678         :u32 = 0x10028e9;  /* U+28e9 BRAILLE PATTERN DOTS-14678 */
pub const XKB_KEY_braille_dots_24678         :u32 = 0x10028ea;  /* U+28ea BRAILLE PATTERN DOTS-24678 */
pub const XKB_KEY_braille_dots_124678        :u32 = 0x10028eb;  /* U+28eb BRAILLE PATTERN DOTS-124678 */
pub const XKB_KEY_braille_dots_34678         :u32 = 0x10028ec;  /* U+28ec BRAILLE PATTERN DOTS-34678 */
pub const XKB_KEY_braille_dots_134678        :u32 = 0x10028ed;  /* U+28ed BRAILLE PATTERN DOTS-134678 */
pub const XKB_KEY_braille_dots_234678        :u32 = 0x10028ee;  /* U+28ee BRAILLE PATTERN DOTS-234678 */
pub const XKB_KEY_braille_dots_1234678       :u32 = 0x10028ef;  /* U+28ef BRAILLE PATTERN DOTS-1234678 */
pub const XKB_KEY_braille_dots_5678          :u32 = 0x10028f0;  /* U+28f0 BRAILLE PATTERN DOTS-5678 */
pub const XKB_KEY_braille_dots_15678         :u32 = 0x10028f1;  /* U+28f1 BRAILLE PATTERN DOTS-15678 */
pub const XKB_KEY_braille_dots_25678         :u32 = 0x10028f2;  /* U+28f2 BRAILLE PATTERN DOTS-25678 */
pub const XKB_KEY_braille_dots_125678        :u32 = 0x10028f3;  /* U+28f3 BRAILLE PATTERN DOTS-125678 */
pub const XKB_KEY_braille_dots_35678         :u32 = 0x10028f4;  /* U+28f4 BRAILLE PATTERN DOTS-35678 */
pub const XKB_KEY_braille_dots_135678        :u32 = 0x10028f5;  /* U+28f5 BRAILLE PATTERN DOTS-135678 */
pub const XKB_KEY_braille_dots_235678        :u32 = 0x10028f6;  /* U+28f6 BRAILLE PATTERN DOTS-235678 */
pub const XKB_KEY_braille_dots_1235678       :u32 = 0x10028f7;  /* U+28f7 BRAILLE PATTERN DOTS-1235678 */
pub const XKB_KEY_braille_dots_45678         :u32 = 0x10028f8;  /* U+28f8 BRAILLE PATTERN DOTS-45678 */
pub const XKB_KEY_braille_dots_145678        :u32 = 0x10028f9;  /* U+28f9 BRAILLE PATTERN DOTS-145678 */
pub const XKB_KEY_braille_dots_245678        :u32 = 0x10028fa;  /* U+28fa BRAILLE PATTERN DOTS-245678 */
pub const XKB_KEY_braille_dots_1245678       :u32 = 0x10028fb;  /* U+28fb BRAILLE PATTERN DOTS-1245678 */
pub const XKB_KEY_braille_dots_345678        :u32 = 0x10028fc;  /* U+28fc BRAILLE PATTERN DOTS-345678 */
pub const XKB_KEY_braille_dots_1345678       :u32 = 0x10028fd;  /* U+28fd BRAILLE PATTERN DOTS-1345678 */
pub const XKB_KEY_braille_dots_2345678       :u32 = 0x10028fe;  /* U+28fe BRAILLE PATTERN DOTS-2345678 */
pub const XKB_KEY_braille_dots_12345678      :u32 = 0x10028ff;  /* U+28ff BRAILLE PATTERN DOTS-12345678 */

/*
 * Sinhala (http://unicode.org/charts/PDF/U0D80.pdf)
 * http://www.nongnu.org/sinhala/doc/transliteration/sinhala-transliteration_6.html
 */

pub const XKB_KEY_Sinh_ng            :u32 = 0x1000d82;  /* U+0D82 SINHALA ANUSVARAYA */
pub const XKB_KEY_Sinh_h2            :u32 = 0x1000d83;  /* U+0D83 SINHALA VISARGAYA */
pub const XKB_KEY_Sinh_a             :u32 = 0x1000d85;  /* U+0D85 SINHALA AYANNA */
pub const XKB_KEY_Sinh_aa            :u32 = 0x1000d86;  /* U+0D86 SINHALA AAYANNA */
pub const XKB_KEY_Sinh_ae            :u32 = 0x1000d87;  /* U+0D87 SINHALA AEYANNA */
pub const XKB_KEY_Sinh_aee           :u32 = 0x1000d88;  /* U+0D88 SINHALA AEEYANNA */
pub const XKB_KEY_Sinh_i             :u32 = 0x1000d89;  /* U+0D89 SINHALA IYANNA */
pub const XKB_KEY_Sinh_ii            :u32 = 0x1000d8a;  /* U+0D8A SINHALA IIYANNA */
pub const XKB_KEY_Sinh_u             :u32 = 0x1000d8b;  /* U+0D8B SINHALA UYANNA */
pub const XKB_KEY_Sinh_uu            :u32 = 0x1000d8c;  /* U+0D8C SINHALA UUYANNA */
pub const XKB_KEY_Sinh_ri            :u32 = 0x1000d8d;  /* U+0D8D SINHALA IRUYANNA */
pub const XKB_KEY_Sinh_rii           :u32 = 0x1000d8e;  /* U+0D8E SINHALA IRUUYANNA */
pub const XKB_KEY_Sinh_lu            :u32 = 0x1000d8f;  /* U+0D8F SINHALA ILUYANNA */
pub const XKB_KEY_Sinh_luu           :u32 = 0x1000d90;  /* U+0D90 SINHALA ILUUYANNA */
pub const XKB_KEY_Sinh_e             :u32 = 0x1000d91;  /* U+0D91 SINHALA EYANNA */
pub const XKB_KEY_Sinh_ee            :u32 = 0x1000d92;  /* U+0D92 SINHALA EEYANNA */
pub const XKB_KEY_Sinh_ai            :u32 = 0x1000d93;  /* U+0D93 SINHALA AIYANNA */
pub const XKB_KEY_Sinh_o             :u32 = 0x1000d94;  /* U+0D94 SINHALA OYANNA */
pub const XKB_KEY_Sinh_oo            :u32 = 0x1000d95;  /* U+0D95 SINHALA OOYANNA */
pub const XKB_KEY_Sinh_au            :u32 = 0x1000d96;  /* U+0D96 SINHALA AUYANNA */
pub const XKB_KEY_Sinh_ka            :u32 = 0x1000d9a;  /* U+0D9A SINHALA KAYANNA */
pub const XKB_KEY_Sinh_kha           :u32 = 0x1000d9b;  /* U+0D9B SINHALA MAHA. KAYANNA */
pub const XKB_KEY_Sinh_ga            :u32 = 0x1000d9c;  /* U+0D9C SINHALA GAYANNA */
pub const XKB_KEY_Sinh_gha           :u32 = 0x1000d9d;  /* U+0D9D SINHALA MAHA. GAYANNA */
pub const XKB_KEY_Sinh_ng2           :u32 = 0x1000d9e;  /* U+0D9E SINHALA KANTAJA NAASIKYAYA */
pub const XKB_KEY_Sinh_nga           :u32 = 0x1000d9f;  /* U+0D9F SINHALA SANYAKA GAYANNA */
pub const XKB_KEY_Sinh_ca            :u32 = 0x1000da0;  /* U+0DA0 SINHALA CAYANNA */
pub const XKB_KEY_Sinh_cha           :u32 = 0x1000da1;  /* U+0DA1 SINHALA MAHA. CAYANNA */
pub const XKB_KEY_Sinh_ja            :u32 = 0x1000da2;  /* U+0DA2 SINHALA JAYANNA */
pub const XKB_KEY_Sinh_jha           :u32 = 0x1000da3;  /* U+0DA3 SINHALA MAHA. JAYANNA */
pub const XKB_KEY_Sinh_nya           :u32 = 0x1000da4;  /* U+0DA4 SINHALA TAALUJA NAASIKYAYA */
pub const XKB_KEY_Sinh_jnya          :u32 = 0x1000da5;  /* U+0DA5 SINHALA TAALUJA SANYOOGA NAASIKYAYA */
pub const XKB_KEY_Sinh_nja           :u32 = 0x1000da6;  /* U+0DA6 SINHALA SANYAKA JAYANNA */
pub const XKB_KEY_Sinh_tta           :u32 = 0x1000da7;  /* U+0DA7 SINHALA TTAYANNA */
pub const XKB_KEY_Sinh_ttha          :u32 = 0x1000da8;  /* U+0DA8 SINHALA MAHA. TTAYANNA */
pub const XKB_KEY_Sinh_dda           :u32 = 0x1000da9;  /* U+0DA9 SINHALA DDAYANNA */
pub const XKB_KEY_Sinh_ddha          :u32 = 0x1000daa;  /* U+0DAA SINHALA MAHA. DDAYANNA */
pub const XKB_KEY_Sinh_nna           :u32 = 0x1000dab;  /* U+0DAB SINHALA MUURDHAJA NAYANNA */
pub const XKB_KEY_Sinh_ndda          :u32 = 0x1000dac;  /* U+0DAC SINHALA SANYAKA DDAYANNA */
pub const XKB_KEY_Sinh_tha           :u32 = 0x1000dad;  /* U+0DAD SINHALA TAYANNA */
pub const XKB_KEY_Sinh_thha          :u32 = 0x1000dae;  /* U+0DAE SINHALA MAHA. TAYANNA */
pub const XKB_KEY_Sinh_dha           :u32 = 0x1000daf;  /* U+0DAF SINHALA DAYANNA */
pub const XKB_KEY_Sinh_dhha          :u32 = 0x1000db0;  /* U+0DB0 SINHALA MAHA. DAYANNA */
pub const XKB_KEY_Sinh_na            :u32 = 0x1000db1;  /* U+0DB1 SINHALA DANTAJA NAYANNA */
pub const XKB_KEY_Sinh_ndha          :u32 = 0x1000db3;  /* U+0DB3 SINHALA SANYAKA DAYANNA */
pub const XKB_KEY_Sinh_pa            :u32 = 0x1000db4;  /* U+0DB4 SINHALA PAYANNA */
pub const XKB_KEY_Sinh_pha           :u32 = 0x1000db5;  /* U+0DB5 SINHALA MAHA. PAYANNA */
pub const XKB_KEY_Sinh_ba            :u32 = 0x1000db6;  /* U+0DB6 SINHALA BAYANNA */
pub const XKB_KEY_Sinh_bha           :u32 = 0x1000db7;  /* U+0DB7 SINHALA MAHA. BAYANNA */
pub const XKB_KEY_Sinh_ma            :u32 = 0x1000db8;  /* U+0DB8 SINHALA MAYANNA */
pub const XKB_KEY_Sinh_mba           :u32 = 0x1000db9;  /* U+0DB9 SINHALA AMBA BAYANNA */
pub const XKB_KEY_Sinh_ya            :u32 = 0x1000dba;  /* U+0DBA SINHALA YAYANNA */
pub const XKB_KEY_Sinh_ra            :u32 = 0x1000dbb;  /* U+0DBB SINHALA RAYANNA */
pub const XKB_KEY_Sinh_la            :u32 = 0x1000dbd;  /* U+0DBD SINHALA DANTAJA LAYANNA */
pub const XKB_KEY_Sinh_va            :u32 = 0x1000dc0;  /* U+0DC0 SINHALA VAYANNA */
pub const XKB_KEY_Sinh_sha           :u32 = 0x1000dc1;  /* U+0DC1 SINHALA TAALUJA SAYANNA */
pub const XKB_KEY_Sinh_ssha          :u32 = 0x1000dc2;  /* U+0DC2 SINHALA MUURDHAJA SAYANNA */
pub const XKB_KEY_Sinh_sa            :u32 = 0x1000dc3;  /* U+0DC3 SINHALA DANTAJA SAYANNA */
pub const XKB_KEY_Sinh_ha            :u32 = 0x1000dc4;  /* U+0DC4 SINHALA HAYANNA */
pub const XKB_KEY_Sinh_lla           :u32 = 0x1000dc5;  /* U+0DC5 SINHALA MUURDHAJA LAYANNA */
pub const XKB_KEY_Sinh_fa            :u32 = 0x1000dc6;  /* U+0DC6 SINHALA FAYANNA */
pub const XKB_KEY_Sinh_al            :u32 = 0x1000dca;  /* U+0DCA SINHALA AL-LAKUNA */
pub const XKB_KEY_Sinh_aa2           :u32 = 0x1000dcf;  /* U+0DCF SINHALA AELA-PILLA */
pub const XKB_KEY_Sinh_ae2           :u32 = 0x1000dd0;  /* U+0DD0 SINHALA AEDA-PILLA */
pub const XKB_KEY_Sinh_aee2          :u32 = 0x1000dd1;  /* U+0DD1 SINHALA DIGA AEDA-PILLA */
pub const XKB_KEY_Sinh_i2            :u32 = 0x1000dd2;  /* U+0DD2 SINHALA IS-PILLA */
pub const XKB_KEY_Sinh_ii2           :u32 = 0x1000dd3;  /* U+0DD3 SINHALA DIGA IS-PILLA */
pub const XKB_KEY_Sinh_u2            :u32 = 0x1000dd4;  /* U+0DD4 SINHALA PAA-PILLA */
pub const XKB_KEY_Sinh_uu2           :u32 = 0x1000dd6;  /* U+0DD6 SINHALA DIGA PAA-PILLA */
pub const XKB_KEY_Sinh_ru2           :u32 = 0x1000dd8;  /* U+0DD8 SINHALA GAETTA-PILLA */
pub const XKB_KEY_Sinh_e2            :u32 = 0x1000dd9;  /* U+0DD9 SINHALA KOMBUVA */
pub const XKB_KEY_Sinh_ee2           :u32 = 0x1000dda;  /* U+0DDA SINHALA DIGA KOMBUVA */
pub const XKB_KEY_Sinh_ai2           :u32 = 0x1000ddb;  /* U+0DDB SINHALA KOMBU DEKA */
pub const XKB_KEY_Sinh_o2            :u32 = 0x1000ddc;  /* U+0DDC SINHALA KOMBUVA HAA AELA-PILLA*/
pub const XKB_KEY_Sinh_oo2           :u32 = 0x1000ddd;  /* U+0DDD SINHALA KOMBUVA HAA DIGA AELA-PILLA*/
pub const XKB_KEY_Sinh_au2           :u32 = 0x1000dde;  /* U+0DDE SINHALA KOMBUVA HAA GAYANUKITTA */
pub const XKB_KEY_Sinh_lu2           :u32 = 0x1000ddf;  /* U+0DDF SINHALA GAYANUKITTA */
pub const XKB_KEY_Sinh_ruu2          :u32 = 0x1000df2;  /* U+0DF2 SINHALA DIGA GAETTA-PILLA */
pub const XKB_KEY_Sinh_luu2          :u32 = 0x1000df3;  /* U+0DF3 SINHALA DIGA GAYANUKITTA */
pub const XKB_KEY_Sinh_kunddaliya    :u32 = 0x1000df4;  /* U+0DF4 SINHALA KUNDDALIYA */
/*
 * XFree86 vendor specific keysyms.
 *
 * The XFree86 keysym range is :u32 = 0x10080001; - :u32 = 0x1008FFFF;.
 *
 * X.Org will not be adding to the XF86 set of keysyms, though they have
 * been adopted and are considered a "standard" part of X keysym definitions.
 * XFree86 never properly commented these keysyms, so we have done our
 * best to explain the semantic meaning of these keys.
 *
 * XFree86 has removed their mail archives of the period, that might have
 * shed more light on some of these definitions. Until/unless we resurrect
 * these archives, these are from memory and usage.
 */

/*
 * ModeLock
 *
 * This one is old, and not really used any more since XKB offers this
 * functionality.
 */

pub const XKB_KEY_XF86ModeLock		:u32 = 0x1008FF01;	/* Mode Switch Lock */

/* Backlight controls. */
pub const XKB_KEY_XF86MonBrightnessUp    :u32 = 0x1008FF02;  /* Monitor/panel brightness */
pub const XKB_KEY_XF86MonBrightnessDown  :u32 = 0x1008FF03;  /* Monitor/panel brightness */
pub const XKB_KEY_XF86KbdLightOnOff      :u32 = 0x1008FF04;  /* Keyboards may be lit     */
pub const XKB_KEY_XF86KbdBrightnessUp    :u32 = 0x1008FF05;  /* Keyboards may be lit     */
pub const XKB_KEY_XF86KbdBrightnessDown  :u32 = 0x1008FF06;  /* Keyboards may be lit     */
pub const XKB_KEY_XF86MonBrightnessCycle :u32 = 0x1008FF07;  /* Monitor/panel brightness */

/*
 * Keys found on some "Internet" keyboards.
 */
pub const XKB_KEY_XF86Standby		:u32 = 0x1008FF10;   /* System into standby mode   */
pub const XKB_KEY_XF86AudioLowerVolume	:u32 = 0x1008FF11;   /* Volume control down        */
pub const XKB_KEY_XF86AudioMute	:u32 = 0x1008FF12;   /* Mute sound from the system */
pub const XKB_KEY_XF86AudioRaiseVolume	:u32 = 0x1008FF13;   /* Volume control up          */
pub const XKB_KEY_XF86AudioPlay	:u32 = 0x1008FF14;   /* Start playing of audio >   */
pub const XKB_KEY_XF86AudioStop	:u32 = 0x1008FF15;   /* Stop playing audio         */
pub const XKB_KEY_XF86AudioPrev	:u32 = 0x1008FF16;   /* Previous track             */
pub const XKB_KEY_XF86AudioNext	:u32 = 0x1008FF17;   /* Next track                 */
pub const XKB_KEY_XF86HomePage		:u32 = 0x1008FF18;   /* Display user's home page   */
pub const XKB_KEY_XF86Mail		:u32 = 0x1008FF19;   /* Invoke user's mail program */
pub const XKB_KEY_XF86Start		:u32 = 0x1008FF1A;   /* Start application          */
pub const XKB_KEY_XF86Search		:u32 = 0x1008FF1B;   /* Search                     */
pub const XKB_KEY_XF86AudioRecord	:u32 = 0x1008FF1C;   /* Record audio application   */

/* These are sometimes found on PDA's (e.g. Palm, PocketPC or elsewhere)   */
pub const XKB_KEY_XF86Calculator	:u32 = 0x1008FF1D;   /* Invoke calculator program  */
pub const XKB_KEY_XF86Memo		:u32 = 0x1008FF1E;   /* Invoke Memo taking program */
pub const XKB_KEY_XF86ToDoList		:u32 = 0x1008FF1F;   /* Invoke To Do List program  */
pub const XKB_KEY_XF86Calendar		:u32 = 0x1008FF20;   /* Invoke Calendar program    */
pub const XKB_KEY_XF86PowerDown	:u32 = 0x1008FF21;   /* Deep sleep the system      */
pub const XKB_KEY_XF86ContrastAdjust	:u32 = 0x1008FF22;   /* Adjust screen contrast     */
pub const XKB_KEY_XF86RockerUp		:u32 = 0x1008FF23;   /* Rocker switches exist up   */
pub const XKB_KEY_XF86RockerDown	:u32 = 0x1008FF24;   /* and down                   */
pub const XKB_KEY_XF86RockerEnter	:u32 = 0x1008FF25;   /* and let you press them     */

/* Some more "Internet" keyboard symbols */
pub const XKB_KEY_XF86Back		:u32 = 0x1008FF26;   /* Like back on a browser     */
pub const XKB_KEY_XF86Forward		:u32 = 0x1008FF27;   /* Like forward on a browser  */
pub const XKB_KEY_XF86Stop		:u32 = 0x1008FF28;   /* Stop current operation     */
pub const XKB_KEY_XF86Refresh		:u32 = 0x1008FF29;   /* Refresh the page           */
pub const XKB_KEY_XF86PowerOff		:u32 = 0x1008FF2A;   /* Power off system entirely  */
pub const XKB_KEY_XF86WakeUp		:u32 = 0x1008FF2B;   /* Wake up system from sleep  */
pub const XKB_KEY_XF86Eject            :u32 = 0x1008FF2C;   /* Eject device (e.g. DVD)    */
pub const XKB_KEY_XF86ScreenSaver      :u32 = 0x1008FF2D;   /* Invoke screensaver         */
pub const XKB_KEY_XF86WWW              :u32 = 0x1008FF2E;   /* Invoke web browser         */
pub const XKB_KEY_XF86Sleep            :u32 = 0x1008FF2F;   /* Put system to sleep        */
pub const XKB_KEY_XF86Favorites	:u32 = 0x1008FF30;   /* Show favorite locations    */
pub const XKB_KEY_XF86AudioPause	:u32 = 0x1008FF31;   /* Pause audio playing        */
pub const XKB_KEY_XF86AudioMedia	:u32 = 0x1008FF32;   /* Launch media collection app */
pub const XKB_KEY_XF86MyComputer	:u32 = 0x1008FF33;   /* Display "My Computer" window */
pub const XKB_KEY_XF86VendorHome	:u32 = 0x1008FF34;   /* Display vendor home web site */
pub const XKB_KEY_XF86LightBulb	:u32 = 0x1008FF35;   /* Light bulb keys exist       */
pub const XKB_KEY_XF86Shop		:u32 = 0x1008FF36;   /* Display shopping web site   */
pub const XKB_KEY_XF86History		:u32 = 0x1008FF37;   /* Show history of web surfing */
pub const XKB_KEY_XF86OpenURL		:u32 = 0x1008FF38;   /* Open selected URL           */
pub const XKB_KEY_XF86AddFavorite	:u32 = 0x1008FF39;   /* Add URL to favorites list   */
pub const XKB_KEY_XF86HotLinks		:u32 = 0x1008FF3A;   /* Show "hot" links            */
pub const XKB_KEY_XF86BrightnessAdjust	:u32 = 0x1008FF3B;   /* Invoke brightness adj. UI   */
pub const XKB_KEY_XF86Finance		:u32 = 0x1008FF3C;   /* Display financial site      */
pub const XKB_KEY_XF86Community	:u32 = 0x1008FF3D;   /* Display user's community    */
pub const XKB_KEY_XF86AudioRewind	:u32 = 0x1008FF3E;   /* "rewind" audio track        */
pub const XKB_KEY_XF86BackForward	:u32 = 0x1008FF3F;   /* ??? */
pub const XKB_KEY_XF86Launch0		:u32 = 0x1008FF40;   /* Launch Application          */
pub const XKB_KEY_XF86Launch1		:u32 = 0x1008FF41;   /* Launch Application          */
pub const XKB_KEY_XF86Launch2		:u32 = 0x1008FF42;   /* Launch Application          */
pub const XKB_KEY_XF86Launch3		:u32 = 0x1008FF43;   /* Launch Application          */
pub const XKB_KEY_XF86Launch4		:u32 = 0x1008FF44;   /* Launch Application          */
pub const XKB_KEY_XF86Launch5		:u32 = 0x1008FF45;   /* Launch Application          */
pub const XKB_KEY_XF86Launch6		:u32 = 0x1008FF46;   /* Launch Application          */
pub const XKB_KEY_XF86Launch7		:u32 = 0x1008FF47;   /* Launch Application          */
pub const XKB_KEY_XF86Launch8		:u32 = 0x1008FF48;   /* Launch Application          */
pub const XKB_KEY_XF86Launch9		:u32 = 0x1008FF49;   /* Launch Application          */
pub const XKB_KEY_XF86LaunchA		:u32 = 0x1008FF4A;   /* Launch Application          */
pub const XKB_KEY_XF86LaunchB		:u32 = 0x1008FF4B;   /* Launch Application          */
pub const XKB_KEY_XF86LaunchC		:u32 = 0x1008FF4C;   /* Launch Application          */
pub const XKB_KEY_XF86LaunchD		:u32 = 0x1008FF4D;   /* Launch Application          */
pub const XKB_KEY_XF86LaunchE		:u32 = 0x1008FF4E;   /* Launch Application          */
pub const XKB_KEY_XF86LaunchF		:u32 = 0x1008FF4F;   /* Launch Application          */

pub const XKB_KEY_XF86ApplicationLeft	:u32 = 0x1008FF50;   /* switch to application, left */
pub const XKB_KEY_XF86ApplicationRight	:u32 = 0x1008FF51;   /* switch to application, right*/
pub const XKB_KEY_XF86Book		:u32 = 0x1008FF52;   /* Launch bookreader           */
pub const XKB_KEY_XF86CD		:u32 = 0x1008FF53;   /* Launch CD/DVD player        */
pub const XKB_KEY_XF86Calculater	:u32 = 0x1008FF54;   /* Launch Calculater           */
pub const XKB_KEY_XF86Clear		:u32 = 0x1008FF55;   /* Clear window, screen        */
pub const XKB_KEY_XF86Close		:u32 = 0x1008FF56;   /* Close window                */
pub const XKB_KEY_XF86Copy		:u32 = 0x1008FF57;   /* Copy selection              */
pub const XKB_KEY_XF86Cut		:u32 = 0x1008FF58;   /* Cut selection               */
pub const XKB_KEY_XF86Display		:u32 = 0x1008FF59;   /* Output switch key           */
pub const XKB_KEY_XF86DOS		:u32 = 0x1008FF5A;   /* Launch DOS (emulation)      */
pub const XKB_KEY_XF86Documents	:u32 = 0x1008FF5B;   /* Open documents window       */
pub const XKB_KEY_XF86Excel		:u32 = 0x1008FF5C;   /* Launch spread sheet         */
pub const XKB_KEY_XF86Explorer		:u32 = 0x1008FF5D;   /* Launch file explorer        */
pub const XKB_KEY_XF86Game		:u32 = 0x1008FF5E;   /* Launch game                 */
pub const XKB_KEY_XF86Go		:u32 = 0x1008FF5F;   /* Go to URL                   */
pub const XKB_KEY_XF86iTouch		:u32 = 0x1008FF60;   /* Logitech iTouch- don't use  */
pub const XKB_KEY_XF86LogOff		:u32 = 0x1008FF61;   /* Log off system              */
pub const XKB_KEY_XF86Market		:u32 = 0x1008FF62;   /* ??                          */
pub const XKB_KEY_XF86Meeting		:u32 = 0x1008FF63;   /* enter meeting in calendar   */
pub const XKB_KEY_XF86MenuKB		:u32 = 0x1008FF65;   /* distinguish keyboard from PB */
pub const XKB_KEY_XF86MenuPB		:u32 = 0x1008FF66;   /* distinguish PB from keyboard */
pub const XKB_KEY_XF86MySites		:u32 = 0x1008FF67;   /* Favourites                  */
pub const XKB_KEY_XF86New		:u32 = 0x1008FF68;   /* New (folder, document...    */
pub const XKB_KEY_XF86News		:u32 = 0x1008FF69;   /* News                        */
pub const XKB_KEY_XF86OfficeHome	:u32 = 0x1008FF6A;   /* Office home (old Staroffice)*/
pub const XKB_KEY_XF86Open		:u32 = 0x1008FF6B;   /* Open                        */
pub const XKB_KEY_XF86Option		:u32 = 0x1008FF6C;   /* ?? */
pub const XKB_KEY_XF86Paste		:u32 = 0x1008FF6D;   /* Paste                       */
pub const XKB_KEY_XF86Phone		:u32 = 0x1008FF6E;   /* Launch phone; dial number   */
pub const XKB_KEY_XF86Q		:u32 = 0x1008FF70;   /* Compaq's Q - don't use      */
pub const XKB_KEY_XF86Reply		:u32 = 0x1008FF72;   /* Reply e.g., mail            */
pub const XKB_KEY_XF86Reload		:u32 = 0x1008FF73;   /* Reload web page, file, etc. */
pub const XKB_KEY_XF86RotateWindows	:u32 = 0x1008FF74;   /* Rotate windows e.g. xrandr  */
pub const XKB_KEY_XF86RotationPB	:u32 = 0x1008FF75;   /* don't use                   */
pub const XKB_KEY_XF86RotationKB	:u32 = 0x1008FF76;   /* don't use                   */
pub const XKB_KEY_XF86Save		:u32 = 0x1008FF77;   /* Save (file, document, state */
pub const XKB_KEY_XF86ScrollUp		:u32 = 0x1008FF78;   /* Scroll window/contents up   */
pub const XKB_KEY_XF86ScrollDown	:u32 = 0x1008FF79;   /* Scrool window/contentd down */
pub const XKB_KEY_XF86ScrollClick	:u32 = 0x1008FF7A;   /* Use XKB mousekeys instead   */
pub const XKB_KEY_XF86Send		:u32 = 0x1008FF7B;   /* Send mail, file, object     */
pub const XKB_KEY_XF86Spell		:u32 = 0x1008FF7C;   /* Spell checker               */
pub const XKB_KEY_XF86SplitScreen	:u32 = 0x1008FF7D;   /* Split window or screen      */
pub const XKB_KEY_XF86Support		:u32 = 0x1008FF7E;   /* Get support (??)            */
pub const XKB_KEY_XF86TaskPane		:u32 = 0x1008FF7F;   /* Show tasks */
pub const XKB_KEY_XF86Terminal		:u32 = 0x1008FF80;   /* Launch terminal emulator    */
pub const XKB_KEY_XF86Tools		:u32 = 0x1008FF81;   /* toolbox of desktop/app.     */
pub const XKB_KEY_XF86Travel		:u32 = 0x1008FF82;   /* ?? */
pub const XKB_KEY_XF86UserPB		:u32 = 0x1008FF84;   /* ?? */
pub const XKB_KEY_XF86User1KB		:u32 = 0x1008FF85;   /* ?? */
pub const XKB_KEY_XF86User2KB		:u32 = 0x1008FF86;   /* ?? */
pub const XKB_KEY_XF86Video		:u32 = 0x1008FF87;   /* Launch video player       */
pub const XKB_KEY_XF86WheelButton	:u32 = 0x1008FF88;   /* button from a mouse wheel */
pub const XKB_KEY_XF86Word		:u32 = 0x1008FF89;   /* Launch word processor     */
pub const XKB_KEY_XF86Xfer		:u32 = 0x1008FF8A;
pub const XKB_KEY_XF86ZoomIn		:u32 = 0x1008FF8B;   /* zoom in view, map, etc.   */
pub const XKB_KEY_XF86ZoomOut		:u32 = 0x1008FF8C;   /* zoom out view, map, etc.  */

pub const XKB_KEY_XF86Away		:u32 = 0x1008FF8D;   /* mark yourself as away     */
pub const XKB_KEY_XF86Messenger	:u32 = 0x1008FF8E;   /* as in instant messaging   */
pub const XKB_KEY_XF86WebCam		:u32 = 0x1008FF8F;   /* Launch web camera app.    */
pub const XKB_KEY_XF86MailForward	:u32 = 0x1008FF90;   /* Forward in mail           */
pub const XKB_KEY_XF86Pictures		:u32 = 0x1008FF91;   /* Show pictures             */
pub const XKB_KEY_XF86Music		:u32 = 0x1008FF92;   /* Launch music application  */

pub const XKB_KEY_XF86Battery		:u32 = 0x1008FF93;   /* Display battery information */
pub const XKB_KEY_XF86Bluetooth	:u32 = 0x1008FF94;   /* Enable/disable Bluetooth    */
pub const XKB_KEY_XF86WLAN		:u32 = 0x1008FF95;   /* Enable/disable WLAN         */
pub const XKB_KEY_XF86UWB		:u32 = 0x1008FF96;   /* Enable/disable UWB	    */

pub const XKB_KEY_XF86AudioForward	:u32 = 0x1008FF97;   /* fast-forward audio track    */
pub const XKB_KEY_XF86AudioRepeat	:u32 = 0x1008FF98;   /* toggle repeat mode          */
pub const XKB_KEY_XF86AudioRandomPlay	:u32 = 0x1008FF99;   /* toggle shuffle mode         */
pub const XKB_KEY_XF86Subtitle		:u32 = 0x1008FF9A;   /* cycle through subtitle      */
pub const XKB_KEY_XF86AudioCycleTrack	:u32 = 0x1008FF9B;   /* cycle through audio tracks  */
pub const XKB_KEY_XF86CycleAngle	:u32 = 0x1008FF9C;   /* cycle through angles        */
pub const XKB_KEY_XF86FrameBack	:u32 = 0x1008FF9D;   /* video: go one frame back    */
pub const XKB_KEY_XF86FrameForward	:u32 = 0x1008FF9E;   /* video: go one frame forward */
pub const XKB_KEY_XF86Time		:u32 = 0x1008FF9F;   /* display, or shows an entry for time seeking */
pub const XKB_KEY_XF86Select		:u32 = 0x1008FFA0;   /* Select button on joypads and remotes */
pub const XKB_KEY_XF86View		:u32 = 0x1008FFA1;   /* Show a view options/properties */
pub const XKB_KEY_XF86TopMenu		:u32 = 0x1008FFA2;   /* Go to a top-level menu in a video */

pub const XKB_KEY_XF86Red		:u32 = 0x1008FFA3;   /* Red button                  */
pub const XKB_KEY_XF86Green		:u32 = 0x1008FFA4;   /* Green button                */
pub const XKB_KEY_XF86Yellow		:u32 = 0x1008FFA5;   /* Yellow button               */
pub const XKB_KEY_XF86Blue             :u32 = 0x1008FFA6;   /* Blue button                 */

pub const XKB_KEY_XF86Suspend		:u32 = 0x1008FFA7;   /* Sleep to RAM                */
pub const XKB_KEY_XF86Hibernate	:u32 = 0x1008FFA8;   /* Sleep to disk               */
pub const XKB_KEY_XF86TouchpadToggle	:u32 = 0x1008FFA9;   /* Toggle between touchpad/trackstick */
pub const XKB_KEY_XF86TouchpadOn	:u32 = 0x1008FFB0;   /* The touchpad got switched on */
pub const XKB_KEY_XF86TouchpadOff	:u32 = 0x1008FFB1;   /* The touchpad got switched off */

pub const XKB_KEY_XF86AudioMicMute	:u32 = 0x1008FFB2;   /* Mute the Mic from the system */

pub const XKB_KEY_XF86Keyboard		:u32 = 0x1008FFB3;   /* User defined keyboard related action */

pub const XKB_KEY_XF86WWAN		:u32 = 0x1008FFB4;   /* Toggle WWAN (LTE, UMTS, etc.) radio */
pub const XKB_KEY_XF86RFKill		:u32 = 0x1008FFB5;   /* Toggle radios on/off */

pub const XKB_KEY_XF86AudioPreset	:u32 = 0x1008FFB6;   /* Select equalizer preset, e.g. theatre-mode */

pub const XKB_KEY_XF86RotationLockToggle :u32 = 0x1008FFB7; /* Toggle screen rotation lock on/off */

pub const XKB_KEY_XF86FullScreen	:u32 = 0x1008FFB8;   /* Toggle fullscreen */

/* Keys for special action keys (hot keys) */
/* Virtual terminals on some operating systems */
pub const XKB_KEY_XF86Switch_VT_1	:u32 = 0x1008FE01;
pub const XKB_KEY_XF86Switch_VT_2	:u32 = 0x1008FE02;
pub const XKB_KEY_XF86Switch_VT_3	:u32 = 0x1008FE03;
pub const XKB_KEY_XF86Switch_VT_4	:u32 = 0x1008FE04;
pub const XKB_KEY_XF86Switch_VT_5	:u32 = 0x1008FE05;
pub const XKB_KEY_XF86Switch_VT_6	:u32 = 0x1008FE06;
pub const XKB_KEY_XF86Switch_VT_7	:u32 = 0x1008FE07;
pub const XKB_KEY_XF86Switch_VT_8	:u32 = 0x1008FE08;
pub const XKB_KEY_XF86Switch_VT_9	:u32 = 0x1008FE09;
pub const XKB_KEY_XF86Switch_VT_10	:u32 = 0x1008FE0A;
pub const XKB_KEY_XF86Switch_VT_11	:u32 = 0x1008FE0B;
pub const XKB_KEY_XF86Switch_VT_12	:u32 = 0x1008FE0C;

pub const XKB_KEY_XF86Ungrab		:u32 = 0x1008FE20;   /* force ungrab               */
pub const XKB_KEY_XF86ClearGrab	:u32 = 0x1008FE21;   /* kill application with grab */
pub const XKB_KEY_XF86Next_VMode	:u32 = 0x1008FE22;   /* next video mode available  */
pub const XKB_KEY_XF86Prev_VMode	:u32 = 0x1008FE23;   /* prev. video mode available */
pub const XKB_KEY_XF86LogWindowTree	:u32 = 0x1008FE24;   /* print window tree to log   */
pub const XKB_KEY_XF86LogGrabInfo	:u32 = 0x1008FE25;   /* print all active grabs to log */
/*
 * Copyright (c) 1991, Oracle and/or its affiliates. All rights reserved.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice (including the next
 * paragraph) shall be included in all copies or substantial portions of the
 * Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
 * THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 */
/************************************************************

Copyright 1991, 1998  The Open Group

Permission to use, copy, modify, distribute, and sell this software and its
documentation for any purpose is hereby granted without fee, provided that
the above copyright notice appear in all copies and that both that
copyright notice and this permission notice appear in supporting
documentation.

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
OPEN GROUP BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN
AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of The Open Group shall not be
used in advertising or otherwise to promote the sale, use or other dealings
in this Software without prior written authorization from The Open Group.

***********************************************************/

/*
 * Floating Accent
 */

pub const XKB_KEY_SunFA_Grave		:u32 = 0x1005FF00;
pub const XKB_KEY_SunFA_Circum		:u32 = 0x1005FF01;
pub const XKB_KEY_SunFA_Tilde		:u32 = 0x1005FF02;
pub const XKB_KEY_SunFA_Acute		:u32 = 0x1005FF03;
pub const XKB_KEY_SunFA_Diaeresis	:u32 = 0x1005FF04;
pub const XKB_KEY_SunFA_Cedilla	:u32 = 0x1005FF05;

/*
 * Miscellaneous Functions
 */

pub const XKB_KEY_SunF36		:u32 = 0x1005FF10;	/* Labeled F11 */
pub const XKB_KEY_SunF37		:u32 = 0x1005FF11;	/* Labeled F12 */

pub const XKB_KEY_SunSys_Req   	:u32 = 0x1005FF60;
pub const XKB_KEY_SunPrint_Screen	:u32 = 0x0000FF61;	/* Same as XK_Print */

/*
 * International & Multi-Key Character Composition
 */

pub const XKB_KEY_SunCompose		:u32 = 0x0000FF20;	/* Same as XK_Multi_key */
pub const XKB_KEY_SunAltGraph		:u32 = 0x0000FF7E;	/* Same as XK_Mode_switch */

/*
 * Cursor Control
 */

pub const XKB_KEY_SunPageUp		:u32 = 0x0000FF55; 	/* Same as XK_Prior */
pub const XKB_KEY_SunPageDown		:u32 = 0x0000FF56;	/* Same as XK_Next */

/*
 * Open Look Functions
 */

pub const XKB_KEY_SunUndo		:u32 = 0x0000FF65;	/* Same as XK_Undo */
pub const XKB_KEY_SunAgain		:u32 = 0x0000FF66;	/* Same as XK_Redo */
pub const XKB_KEY_SunFind		:u32 = 0x0000FF68;	/* Same as XK_Find */
pub const XKB_KEY_SunStop		:u32 = 0x0000FF69;	/* Same as XK_Cancel */
pub const XKB_KEY_SunProps		:u32 = 0x1005FF70;
pub const XKB_KEY_SunFront		:u32 = 0x1005FF71;
pub const XKB_KEY_SunCopy		:u32 = 0x1005FF72;
pub const XKB_KEY_SunOpen		:u32 = 0x1005FF73;
pub const XKB_KEY_SunPaste		:u32 = 0x1005FF74;
pub const XKB_KEY_SunCut		:u32 = 0x1005FF75;

pub const XKB_KEY_SunPowerSwitch		:u32 = 0x1005FF76;
pub const XKB_KEY_SunAudioLowerVolume		:u32 = 0x1005FF77;
pub const XKB_KEY_SunAudioMute			:u32 = 0x1005FF78;
pub const XKB_KEY_SunAudioRaiseVolume		:u32 = 0x1005FF79;
pub const XKB_KEY_SunVideoDegauss		:u32 = 0x1005FF7A;
pub const XKB_KEY_SunVideoLowerBrightness	:u32 = 0x1005FF7B;
pub const XKB_KEY_SunVideoRaiseBrightness	:u32 = 0x1005FF7C;
pub const XKB_KEY_SunPowerSwitchShift		:u32 = 0x1005FF7D;
/***********************************************************

Copyright 1988, 1998  The Open Group

Permission to use, copy, modify, distribute, and sell this software and its
documentation for any purpose is hereby granted without fee, provided that
the above copyright notice appear in all copies and that both that
copyright notice and this permission notice appear in supporting
documentation.

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
OPEN GROUP BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN
AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of The Open Group shall not be
used in advertising or otherwise to promote the sale, use or other dealings
in this Software without prior written authorization from The Open Group.


Copyright 1988 by Digital Equipment Corporation, Maynard, Massachusetts.

                        All Rights Reserved

Permission to use, copy, modify, and distribute this software and its
documentation for any purpose and without fee is hereby granted,
provided that the above copyright notice appear in all copies and that
both that copyright notice and this permission notice appear in
supporting documentation, and that the name of Digital not be
used in advertising or publicity pertaining to distribution of the
software without specific, written prior permission.

DIGITAL DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE, INCLUDING
ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS, IN NO EVENT SHALL
DIGITAL BE LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR
ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS
SOFTWARE.

******************************************************************/

/*
 * DEC private keysyms
 * (29th bit set)
 */

/* two-key compose sequence initiators, chosen to map to Latin1 characters */

pub const XKB_KEY_Dring_accent         :u32 = 0x1000FEB0;
pub const XKB_KEY_Dcircumflex_accent   :u32 = 0x1000FE5E;
pub const XKB_KEY_Dcedilla_accent      :u32 = 0x1000FE2C;
pub const XKB_KEY_Dacute_accent        :u32 = 0x1000FE27;
pub const XKB_KEY_Dgrave_accent        :u32 = 0x1000FE60;
pub const XKB_KEY_Dtilde               :u32 = 0x1000FE7E;
pub const XKB_KEY_Ddiaeresis           :u32 = 0x1000FE22;

/* special keysym for LK2** "Remove" key on editing keypad */

pub const XKB_KEY_DRemove	:u32 = 0x1000FF00;   /* Remove */
/*

Copyright 1987, 1998  The Open Group

Permission to use, copy, modify, distribute, and sell this software and its
documentation for any purpose is hereby granted without fee, provided that
the above copyright notice appear in all copies and that both that
copyright notice and this permission notice appear in supporting
documentation.

The above copyright notice and this permission notice shall be included
in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE OPEN GROUP BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of The Open Group shall
not be used in advertising or otherwise to promote the sale, use or
other dealings in this Software without prior written authorization
from The Open Group.

Copyright 1987 by Digital Equipment Corporation, Maynard, Massachusetts,

                        All Rights Reserved

Permission to use, copy, modify, and distribute this software and its
documentation for any purpose and without fee is hereby granted,
provided that the above copyright notice appear in all copies and that
both that copyright notice and this permission notice appear in
supporting documentation, and that the names of Hewlett Packard
or Digital not be
used in advertising or publicity pertaining to distribution of the
software without specific, written prior permission.

DIGITAL DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE, INCLUDING
ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS, IN NO EVENT SHALL
DIGITAL BE LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR
ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS
SOFTWARE.

HEWLETT-PACKARD MAKES NO WARRANTY OF ANY KIND WITH REGARD
TO THIS SOFWARE, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
PURPOSE.  Hewlett-Packard shall not be liable for errors
contained herein or direct, indirect, special, incidental or
consequential damages in connection with the furnishing,
performance, or use of this material.

*/



pub const XKB_KEY_hpClearLine		:u32 = 0x1000FF6F;
pub const XKB_KEY_hpInsertLine		:u32 = 0x1000FF70;
pub const XKB_KEY_hpDeleteLine		:u32 = 0x1000FF71;
pub const XKB_KEY_hpInsertChar		:u32 = 0x1000FF72;
pub const XKB_KEY_hpDeleteChar		:u32 = 0x1000FF73;
pub const XKB_KEY_hpBackTab		:u32 = 0x1000FF74;
pub const XKB_KEY_hpKP_BackTab		:u32 = 0x1000FF75;
pub const XKB_KEY_hpModelock1		:u32 = 0x1000FF48;
pub const XKB_KEY_hpModelock2		:u32 = 0x1000FF49;
pub const XKB_KEY_hpReset		:u32 = 0x1000FF6C;
pub const XKB_KEY_hpSystem		:u32 = 0x1000FF6D;
pub const XKB_KEY_hpUser		:u32 = 0x1000FF6E;
pub const XKB_KEY_hpmute_acute		:u32 = 0x100000A8;
pub const XKB_KEY_hpmute_grave		:u32 = 0x100000A9;
pub const XKB_KEY_hpmute_asciicircum	:u32 = 0x100000AA;
pub const XKB_KEY_hpmute_diaeresis	:u32 = 0x100000AB;
pub const XKB_KEY_hpmute_asciitilde	:u32 = 0x100000AC;
pub const XKB_KEY_hplira		:u32 = 0x100000AF;
pub const XKB_KEY_hpguilder		:u32 = 0x100000BE;
pub const XKB_KEY_hpYdiaeresis		:u32 = 0x100000EE;
pub const XKB_KEY_hpIO			:u32 = 0x100000EE;
pub const XKB_KEY_hplongminus		:u32 = 0x100000F6;
pub const XKB_KEY_hpblock		:u32 = 0x100000FC;



pub const XKB_KEY_osfCopy		:u32 = 0x1004FF02;
pub const XKB_KEY_osfCut		:u32 = 0x1004FF03;
pub const XKB_KEY_osfPaste		:u32 = 0x1004FF04;
pub const XKB_KEY_osfBackTab		:u32 = 0x1004FF07;
pub const XKB_KEY_osfBackSpace		:u32 = 0x1004FF08;
pub const XKB_KEY_osfClear		:u32 = 0x1004FF0B;
pub const XKB_KEY_osfEscape		:u32 = 0x1004FF1B;
pub const XKB_KEY_osfAddMode		:u32 = 0x1004FF31;
pub const XKB_KEY_osfPrimaryPaste	:u32 = 0x1004FF32;
pub const XKB_KEY_osfQuickPaste	:u32 = 0x1004FF33;
pub const XKB_KEY_osfPageLeft		:u32 = 0x1004FF40;
pub const XKB_KEY_osfPageUp		:u32 = 0x1004FF41;
pub const XKB_KEY_osfPageDown		:u32 = 0x1004FF42;
pub const XKB_KEY_osfPageRight		:u32 = 0x1004FF43;
pub const XKB_KEY_osfActivate		:u32 = 0x1004FF44;
pub const XKB_KEY_osfMenuBar		:u32 = 0x1004FF45;
pub const XKB_KEY_osfLeft		:u32 = 0x1004FF51;
pub const XKB_KEY_osfUp		:u32 = 0x1004FF52;
pub const XKB_KEY_osfRight		:u32 = 0x1004FF53;
pub const XKB_KEY_osfDown		:u32 = 0x1004FF54;
pub const XKB_KEY_osfEndLine		:u32 = 0x1004FF57;
pub const XKB_KEY_osfBeginLine		:u32 = 0x1004FF58;
pub const XKB_KEY_osfEndData		:u32 = 0x1004FF59;
pub const XKB_KEY_osfBeginData		:u32 = 0x1004FF5A;
pub const XKB_KEY_osfPrevMenu		:u32 = 0x1004FF5B;
pub const XKB_KEY_osfNextMenu		:u32 = 0x1004FF5C;
pub const XKB_KEY_osfPrevField		:u32 = 0x1004FF5D;
pub const XKB_KEY_osfNextField		:u32 = 0x1004FF5E;
pub const XKB_KEY_osfSelect		:u32 = 0x1004FF60;
pub const XKB_KEY_osfInsert		:u32 = 0x1004FF63;
pub const XKB_KEY_osfUndo		:u32 = 0x1004FF65;
pub const XKB_KEY_osfMenu		:u32 = 0x1004FF67;
pub const XKB_KEY_osfCancel		:u32 = 0x1004FF69;
pub const XKB_KEY_osfHelp		:u32 = 0x1004FF6A;
pub const XKB_KEY_osfSelectAll		:u32 = 0x1004FF71;
pub const XKB_KEY_osfDeselectAll	:u32 = 0x1004FF72;
pub const XKB_KEY_osfReselect		:u32 = 0x1004FF73;
pub const XKB_KEY_osfExtend		:u32 = 0x1004FF74;
pub const XKB_KEY_osfRestore		:u32 = 0x1004FF78;
pub const XKB_KEY_osfDelete		:u32 = 0x1004FFFF;



/**************************************************************
 * The use of the following macros is deprecated.
 * They are listed below only for backwards compatibility.
 */
pub const XKB_KEY_Reset                :u32 = 0x1000FF6C;
pub const XKB_KEY_System               :u32 = 0x1000FF6D;
pub const XKB_KEY_User                 :u32 = 0x1000FF6E;
pub const XKB_KEY_ClearLine            :u32 = 0x1000FF6F;
pub const XKB_KEY_InsertLine           :u32 = 0x1000FF70;
pub const XKB_KEY_DeleteLine           :u32 = 0x1000FF71;
pub const XKB_KEY_InsertChar           :u32 = 0x1000FF72;
pub const XKB_KEY_DeleteChar           :u32 = 0x1000FF73;
pub const XKB_KEY_BackTab              :u32 = 0x1000FF74;
pub const XKB_KEY_KP_BackTab           :u32 = 0x1000FF75;
pub const XKB_KEY_Ext16bit_L           :u32 = 0x1000FF76;
pub const XKB_KEY_Ext16bit_R           :u32 = 0x1000FF77;
pub const XKB_KEY_mute_acute           :u32 = 0x100000a8;
pub const XKB_KEY_mute_grave           :u32 = 0x100000a9;
pub const XKB_KEY_mute_asciicircum     :u32 = 0x100000aa;
pub const XKB_KEY_mute_diaeresis       :u32 = 0x100000ab;
pub const XKB_KEY_mute_asciitilde      :u32 = 0x100000ac;
pub const XKB_KEY_lira                 :u32 = 0x100000af;
pub const XKB_KEY_guilder              :u32 = 0x100000be;
pub const XKB_KEY_IO                   :u32 = 0x100000ee;
pub const XKB_KEY_longminus            :u32 = 0x100000f6;
pub const XKB_KEY_block                :u32 = 0x100000fc;

