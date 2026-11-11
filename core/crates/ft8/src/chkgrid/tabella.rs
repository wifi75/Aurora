// Generato da `chkgrid2rs.py` traducendo in modo meccanico `lib/ft8var/chkgridvar.f90` di WSJT-X 3.3.0-beta6 (GPL-3.0; la tabella viene dal
// decoder di JTDX). Non si modifica a mano: si rigenera dal sorgente. Le condizioni sono quelle del Fortran: sottostringhe 1..n e confronti tra
// caratteri come in Fortran (spazi a destra). `true` = il Fortran ha fatto `return` (tutto deciso).

use super::{eq, gt, len_trim, lt, ne, sub, Flags};

#[allow(clippy::all, clippy::pedantic, unused_parens, unreachable_code, unused_variables)]
pub(super) fn run(c: &[u8; 12], g: &[u8; 12], f: &mut Flags) -> bool {
    f.chkcall = false;
    f.gvalid = false;
    f.wrongcall = false;
    if gt(sub(g, 1, 1), b"@") && lt(sub(g, 1, 1), b"J") {
        if eq(sub(g, 1, 1), b"A") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"E")) || eq(sub(g, 2, 2), b"M") || eq(sub(g, 2, 2), b"N") || eq(sub(g, 2, 2), b"Q") || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"B") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"G")) || eq(sub(g, 2, 2), b"M") || eq(sub(g, 2, 2), b"N") || eq(sub(g, 2, 2), b"Q") || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"C") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"G")) || (eq(sub(g, 2, 2), b"G") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 2, 2), b"I") && gt(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"1")) || (gt(sub(g, 2, 2), b"I") && lt(sub(g, 2, 2), b"M")) || eq(sub(g, 2, 2), b"Q") || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"D") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"G")) || (gt(sub(g, 2, 2), b"G") && lt(sub(g, 2, 2), b"K")) || (eq(sub(g, 2, 2), b"G") && ne(sub(g, 3, 4), b"52") && ne(sub(g, 3, 4), b"73")) || eq(sub(g, 2, 2), b"Q") || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"E") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"C")) || (eq(sub(g, 2, 2), b"C") && ne(sub(g, 3, 4), b"41")) || (eq(sub(g, 2, 2), b"F") && ne(sub(g, 3, 4), b"96")) || (gt(sub(g, 2, 2), b"C") && lt(sub(g, 2, 2), b"F")) || (eq(sub(g, 2, 2), b"G") && ne(sub(g, 3, 4), b"93")) || eq(sub(g, 2, 2), b"H") || eq(sub(g, 2, 2), b"Q") || (eq(sub(g, 2, 2), b"R") && ne(sub(g, 3, 4), b"60"))) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"F") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"D")) || (eq(sub(g, 2, 2), b"Q") && lt(sub(g, 3, 3), b"3")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"G") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"C")) || (gt(sub(g, 2, 2), b"K") && lt(sub(g, 2, 2), b"N")) || (eq(sub(g, 2, 2), b"K") && gt(sub(g, 3, 3), b"0")) || (eq(sub(g, 2, 2), b"N") && gt(sub(g, 3, 3), b"3")) || (eq(sub(g, 2, 2), b"Q") && gt(sub(g, 4, 4), b"4")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"H") && (eq(sub(g, 2, 2), b"A") || (eq(sub(g, 2, 2), b"B") && ne(sub(g, 3, 4), b"22")) || eq(sub(g, 2, 2), b"C") || (gt(sub(g, 2, 2), b"D") && lt(sub(g, 2, 2), b"H")) || eq(sub(g, 2, 2), b"J") || eq(sub(g, 2, 2), b"L") || eq(sub(g, 2, 2), b"N") || eq(sub(g, 2, 2), b"O") || (eq(sub(g, 2, 2), b"K") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 2, 2), b"M") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 2, 2), b"Q") && gt(sub(g, 4, 4), b"0")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"I") && (eq(sub(g, 2, 2), b"A") || (gt(sub(g, 2, 2), b"B") && lt(sub(g, 2, 2), b"F")) || (eq(sub(g, 2, 2), b"B") && ne(sub(g, 3, 4), b"59")) || (eq(sub(g, 2, 2), b"F") && ne(sub(g, 3, 4), b"32")) || eq(sub(g, 2, 2), b"G") || (eq(sub(g, 2, 2), b"H") && ne(sub(g, 3, 3), b"7")) || (eq(sub(g, 2, 2), b"I") && ne(sub(g, 3, 4), b"22")) || (eq(sub(g, 2, 2), b"O") && lt(sub(g, 3, 3), b"3")) || (eq(sub(g, 2, 2), b"P") && gt(sub(g, 4, 4), b"7")) || (eq(sub(g, 2, 2), b"Q") && gt(sub(g, 4, 4), b"1")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        }
    } else if gt(sub(g, 1, 1), b"I") && lt(sub(g, 1, 1), b"S") {
        if eq(sub(g, 1, 1), b"J") && (eq(sub(g, 2, 2), b"C") || (eq(sub(g, 2, 2), b"A") && ne(sub(g, 3, 4), b"00")) || (eq(sub(g, 2, 2), b"B") && ne(sub(g, 3, 4), b"59")) || (eq(sub(g, 2, 2), b"D") && ne(sub(g, 3, 4), b"15")) || eq(sub(g, 2, 2), b"E") || (eq(sub(g, 2, 2), b"F") && lt(sub(g, 3, 3), b"8")) || (eq(sub(g, 2, 2), b"G") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 2, 2), b"H") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 2, 2), b"I") && lt(sub(g, 3, 3), b"2")) || (eq(sub(g, 2, 2), b"R") && gt(sub(g, 4, 4), b"0"))) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"K") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"C")) || (eq(sub(g, 2, 2), b"C") && ne(sub(g, 3, 4), b"90")) || eq(sub(g, 2, 2), b"D") || (eq(sub(g, 2, 2), b"E") && ne(sub(g, 3, 4), b"83") && ne(sub(g, 3, 4), b"93")) || (eq(sub(g, 2, 2), b"F") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 2, 2), b"R") && gt(sub(g, 4, 4), b"0"))) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"L") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"E")) || (eq(sub(g, 2, 2), b"E") && ne(sub(g, 3, 4), b"53") && ne(sub(g, 3, 4), b"54") && ne(sub(g, 3, 4), b"63")) || eq(sub(g, 2, 2), b"F") || (eq(sub(g, 2, 2), b"G") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 2, 2), b"J") && gt(sub(g, 3, 3), b"5")) || (eq(sub(g, 2, 2), b"Q") && lt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 2, 2), b"R") && gt(sub(g, 4, 4), b"1"))) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"M") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"D")) || (eq(sub(g, 2, 2), b"D") && ne(sub(g, 3, 4), b"66") && ne(sub(g, 3, 4), b"67") && ne(sub(g, 3, 4), b"49")) || (eq(sub(g, 2, 2), b"E") && ne(sub(g, 3, 4), b"40") && ne(sub(g, 3, 4), b"41") && ne(sub(g, 3, 4), b"50")) || (eq(sub(g, 2, 2), b"F") && ne(sub(g, 3, 4), b"81") && ne(sub(g, 3, 4), b"82")) || eq(sub(g, 2, 2), b"G") || (eq(sub(g, 2, 2), b"H") && ne(sub(g, 3, 4), b"10")) || (eq(sub(g, 2, 2), b"I") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 2, 2), b"J") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 2, 2), b"K") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 2, 2), b"R") && gt(sub(g, 4, 4), b"1"))) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"N") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"H")) || (eq(sub(g, 2, 2), b"H") && ne(sub(g, 3, 4), b"87") && ne(sub(g, 3, 4), b"88")) || (eq(sub(g, 2, 2), b"I") && lt(sub(g, 3, 3), b"9") && ne(sub(g, 3, 4), b"89")) || (eq(sub(g, 2, 2), b"J") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 2, 2), b"R") && gt(sub(g, 4, 4), b"1"))) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"O") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"F")) || (eq(sub(g, 2, 2), b"F") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 2, 2), b"G") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 2, 2), b"H") && ne(sub(g, 3, 4), b"90") && ne(sub(g, 3, 4), b"92") && ne(sub(g, 3, 4), b"29") && ne(sub(g, 3, 4), b"99")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"P") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"F")) || (eq(sub(g, 2, 2), b"F") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 2, 2), b"K") && gt(sub(g, 3, 3), b"3") && ne(sub(g, 3, 4), b"90")) || (eq(sub(g, 2, 2), b"Q") && gt(sub(g, 4, 4), b"6")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"Q") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"D")) || (eq(sub(g, 2, 2), b"D") && ne(sub(g, 3, 4), b"94") && ne(sub(g, 3, 4), b"95")) || (eq(sub(g, 2, 2), b"E") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 2, 2), b"F") && gt(sub(g, 3, 3), b"6") && ne(sub(g, 3, 4), b"98")) || (eq(sub(g, 2, 2), b"K") && gt(sub(g, 3, 3), b"2") && ne(sub(g, 3, 4), b"36")) || (eq(sub(g, 2, 2), b"L") && gt(sub(g, 3, 3), b"2") && ne(sub(g, 3, 4), b"64") && ne(sub(g, 3, 4), b"74")) || (eq(sub(g, 2, 2), b"M") && gt(sub(g, 3, 3), b"0") && ne(sub(g, 3, 4), b"19")) || (eq(sub(g, 2, 2), b"N") && gt(sub(g, 3, 3), b"7")) || (eq(sub(g, 2, 2), b"Q") && gt(sub(g, 4, 4), b"7")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        } else if eq(sub(g, 1, 1), b"R") && ((gt(sub(g, 2, 2), b"@") && lt(sub(g, 2, 2), b"D")) || (eq(sub(g, 2, 2), b"D") && ne(sub(g, 3, 4), b"47") && ne(sub(g, 3, 4), b"29") && ne(sub(g, 3, 4), b"39")) || (eq(sub(g, 2, 2), b"K") && gt(sub(g, 4, 4), b"4") && ne(sub(g, 3, 4), b"39")) || (gt(sub(g, 2, 2), b"K") && lt(sub(g, 2, 2), b"O")) || (eq(sub(g, 2, 2), b"O") && eq(sub(g, 4, 4), b"0")) || (eq(sub(g, 2, 2), b"Q") && gt(sub(g, 4, 4), b"1")) || eq(sub(g, 2, 2), b"R")) {
            f.chkcall = true;
        }
    } else if gt(sub(g, 1, 1), b"/") && lt(sub(g, 1, 1), b":") {
        f.chkcall = true;
    }
    if !f.chkcall {
        if gt(sub(c, 1, 1), b"K") && lt(sub(c, 1, 1), b"[") {
            if block_2(c, g, f) { return true; } if block_4(c, g, f) { return true; } return true;
        } else if gt(sub(c, 1, 1), b"/") && lt(sub(c, 1, 1), b":") {
            if block_4(c, g, f) { return true; } return true;
        }
        if eq(sub(c, 1, 1), b"A") {
            if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"M") {
                if (eq(sub(g, 1, 2), b"FM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"DL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"EM") || eq(sub(g, 1, 2), b"DM") || eq(sub(g, 1, 2), b"EN") || (eq(sub(g, 1, 2), b"CM") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"DN") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 2), b"BP") || (eq(sub(g, 1, 2), b"BO") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AP") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 1, 4), b"FK28") || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"BL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"BK") && (eq(sub(g, 3, 4), b"19") || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"AL") && (eq(sub(g, 3, 4), b"91") || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"27") || eq(sub(g, 3, 4), b"36") || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"93"))) || (eq(sub(g, 1, 3), b"QK2") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"RO") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 3, 4), b"63"))) || eq(sub(g, 1, 4), b"AP30") || eq(sub(g, 1, 4), b"QK36") || eq(sub(g, 1, 4), b"QL20") || eq(sub(g, 1, 4), b"RK39") || eq(sub(g, 1, 4), b"AK56") || eq(sub(g, 1, 4), b"AH48") || eq(sub(g, 1, 4), b"JA00") || eq(sub(g, 1, 4), b"RB32") || eq(sub(g, 1, 4), b"CP00") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"FM") && (eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"24")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"EL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"28") && ne(sub(g, 3, 4), b"58")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"14")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"DL") && (eq(sub(g, 3, 4), b"78") || eq(sub(g, 3, 4), b"88")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"DM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"EN") && ((eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || ((eq(sub(g, 3, 3), b"8") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && eq(sub(g, 4, 4), b"9") && ne(sub(g, 3, 3), b"2"))) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"CM") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"FN") && ((eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"52")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"CO") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"39")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"BO") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"14") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"35")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"AP") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"AO") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":") && ne(sub(g, 4, 4), b"6")) || ((eq(sub(g, 3, 3), b"6") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"97") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"87") || eq(sub(g, 3, 4), b"31")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"BL") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"RO") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"71")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 2), b"AX") {
                if (eq(sub(g, 1, 2), b"QG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 1, 2), b"QF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 1, 2), b"PF") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OF") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"QE") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"19"))) || (eq(sub(g, 1, 2), b"QH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"OG") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 1, 2), b"PG") || eq(sub(g, 1, 2), b"PH") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"QF") && eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7") {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"PF") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"7")) || eq(sub(g, 3, 4), b"57")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 4), b"OF74") {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"QH") && ((eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"09")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"OG") && (eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"69")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"PH") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":") && ne(sub(g, 4, 4), b"5") && ne(sub(g, 4, 4), b"6")) || (gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"89")) {
                        f.gvalid = false;
                    }
                }
            } else if gt(sub(c, 2, 2), b"O") && lt(sub(c, 2, 2), b"T") {
                if (eq(sub(g, 1, 2), b"MM") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85"))) || (eq(sub(g, 1, 2), b"ML") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"33"))) { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"MM") && eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7") {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"ML") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"07")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 1), b"A") && gt(sub(c, 2, 2), b"L") && lt(sub(c, 2, 2), b"P") {
                if (eq(sub(g, 1, 2), b"IN") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4") && ne(sub(g, 3, 4), b"50")) || (eq(sub(g, 1, 2), b"IM") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"85"))) || (eq(sub(g, 1, 2), b"JN") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"20"))) || (eq(sub(g, 1, 2), b"JM") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09") || eq(sub(g, 3, 4), b"19") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"IL") && ((eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || (eq(sub(g, 4, 4), b"8") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || eq(sub(g, 3, 4), b"39"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"A2") {
                if (eq(sub(g, 1, 2), b"KG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"KH") && (eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"30"))) || (eq(sub(g, 1, 2), b"KG") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"47"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"A3") {
                if (eq(sub(g, 1, 2), b"AG") && (eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"29") || eq(sub(g, 3, 4), b"06") || eq(sub(g, 3, 4), b"07"))) || (eq(sub(g, 1, 2), b"AH") && (eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"A4") {
                if (eq(sub(g, 1, 2), b"LL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"LK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"A5") {
                if eq(sub(g, 1, 2), b"NL") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"9") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"A6") {
                if eq(sub(g, 1, 2), b"LL") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6")) || eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"54")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"A7") {
                if (eq(sub(g, 1, 3), b"LL5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 1, 4), b"LL65") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"A9") {
                if eq(sub(g, 1, 3), b"LL5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"7") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"AY") || eq(sub(c, 1, 2), b"AZ") {
                if (eq(sub(g, 1, 2), b"GF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FF") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FG") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"GG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"FE") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"9")) || (eq(sub(g, 1, 2), b"FD") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"GC07") || eq(sub(g, 1, 4), b"GC16") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"GF") && (eq(sub(g, 3, 4), b"16") || eq(sub(g, 3, 4), b"17")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"FG") && (eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"57")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"GG") && (eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"35")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"FE") && ((eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"70") || eq(sub(g, 3, 4), b"73")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"FD") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":"))) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 1), b"A") && gt(sub(c, 2, 2), b"S") && lt(sub(c, 2, 2), b"X") {
                if (eq(sub(g, 1, 2), b"MJ") && (eq(sub(g, 3, 4), b"99") || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"68"))) || (eq(sub(g, 1, 2), b"MK") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"52"))) || (eq(sub(g, 1, 2), b"NK") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"39"))) || (eq(sub(g, 1, 2), b"ML") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 1, 4), b"NM00") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"MK") && eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5") {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"NK") && (eq(sub(g, 3, 4), b"26") || eq(sub(g, 3, 4), b"27")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"ML") && ((gt(sub(g, 3, 3), b"4") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"5") || eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"NL") && ((eq(sub(g, 4, 4), b"9") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 4, 4), b"8") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"40")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"MM") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"62")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 2), b"A8") {
                if eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"67") && ne(sub(g, 3, 4), b"67") { f.gvalid = true; }
            }
        } else if eq(sub(c, 1, 1), b"B") {
            if gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":") && gt(sub(c, 3, 3), b"/") && lt(sub(c, 3, 3), b":") { return true; }
            if eq(sub(g, 1, 2), b"OL") || (eq(sub(g, 1, 2), b"PL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2")) || eq(sub(g, 1, 2), b"OM") || (eq(sub(g, 1, 2), b"PM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || (eq(sub(g, 1, 2), b"PN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8")) || (eq(sub(g, 1, 2), b"PO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"OK") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59"))) || eq(sub(g, 1, 2), b"ON") || eq(sub(g, 1, 4), b"OO90") || eq(sub(g, 1, 4), b"OO91") || eq(sub(g, 1, 2), b"NN") || eq(sub(g, 1, 2), b"NM") || (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"MN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) { f.gvalid = true; }
            if f.gvalid {
                if (gt(sub(c, 2, 2), b"L") && lt(sub(c, 2, 2), b"R")) || (gt(sub(c, 2, 2), b"T") && lt(sub(c, 2, 2), b"Y")) {
                    if ne(sub(g, 1, 4), b"PL04") && ne(sub(g, 1, 4), b"PL03") && ne(sub(g, 1, 4), b"PL05") && ne(sub(g, 1, 4), b"PL02") && ne(sub(g, 1, 4), b"PL01") && ne(sub(g, 1, 4), b"OL93") && ne(sub(g, 1, 4), b"PL15") && ne(sub(g, 1, 4), b"OL80") { f.gvalid = false; }
                }
            }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"NL") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7") && ne(sub(g, 3, 4), b"83") && ne(sub(g, 3, 4), b"84") && ne(sub(g, 3, 4), b"85")) || eq(sub(g, 3, 4), b"77") || eq(sub(g, 3, 4), b"87")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"ON") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PN") && ((eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"MM") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OL") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && eq(sub(g, 4, 4), b"0")) || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"21")) {
                    f.gvalid = false;
                }
            }
            if !f.gvalid {
                if eq(sub(c, 3, 4), b"9S") && ((gt(sub(c, 2, 2), b"K") && lt(sub(c, 2, 2), b"R")) || (gt(sub(c, 2, 2), b"T") && lt(sub(c, 2, 2), b"Y"))) {
                    if (eq(sub(g, 1, 2), b"OK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
                }
            }
        } else if eq(sub(c, 1, 1), b"C") {
            if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"F") {
                if (eq(sub(g, 1, 2), b"FF") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"FG") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")))) || (eq(sub(g, 1, 2), b"FE") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"FH") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"52"))) || (eq(sub(g, 1, 2), b"FD") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"85"))) || eq(sub(g, 1, 4), b"EF96") || eq(sub(g, 1, 4), b"FF06") || eq(sub(g, 1, 4), b"EG93") || eq(sub(g, 1, 4), b"FG03") || eq(sub(g, 1, 4), b"DG52") || eq(sub(g, 1, 4), b"DG73") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"FF") && eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":") {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"FE") && (eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"41") || eq(sub(g, 3, 4), b"29")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 2), b"CL") || eq(sub(c, 1, 2), b"CM") || eq(sub(c, 1, 2), b"CO") {
                if (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || eq(sub(g, 1, 4), b"FK19") { f.gvalid = true; }
            } else if (gt(sub(c, 2, 2), b"E") && lt(sub(c, 2, 2), b"L")) || eq(sub(c, 1, 2), b"CY") || eq(sub(c, 1, 2), b"CZ") {
                if (eq(sub(g, 1, 2), b"CN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FN") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"EN") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"CO") || (eq(sub(g, 1, 2), b"GN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"FO") || eq(sub(g, 1, 2), b"EO") || eq(sub(g, 1, 2), b"DO") || (eq(sub(g, 1, 2), b"GO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"DN") && gt(sub(g, 4, 4), b"8") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"CP") || eq(sub(g, 1, 2), b"DP") || eq(sub(g, 1, 2), b"EP") || (eq(sub(g, 1, 2), b"FP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"96"))) || (eq(sub(g, 1, 2), b"EQ") && (eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"96"))) || (eq(sub(g, 1, 2), b"FR") && (eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"82"))) || (eq(sub(g, 1, 2), b"FQ") && (eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"12"))) || eq(sub(g, 1, 4), b"GN03") || eq(sub(g, 1, 4), b"FN93") || eq(sub(g, 1, 4), b"ER60") || eq(sub(g, 1, 4), b"DQ10") || eq(sub(g, 1, 4), b"CQ71") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"EO") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"9")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"86")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"CO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"40")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"EP") && ((eq(sub(g, 4, 4), b"0") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 4, 4), b"1") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9")) || (eq(sub(g, 4, 4), b"2") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"8"))) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"CN") && (eq(sub(g, 3, 4), b"68") || eq(sub(g, 3, 4), b"98")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"FP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"04")) {
                        f.gvalid = false;
                    }
                }
            } else if gt(sub(c, 2, 2), b"P") && lt(sub(c, 2, 2), b"U") {
                if (eq(sub(g, 1, 2), b"IN") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"IM") && gt(sub(g, 3, 3), b"4") && ((lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"20"))) || (eq(sub(g, 1, 2), b"HM") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"76"))) { f.gvalid = true; }
            } else if gt(sub(c, 2, 2), b"U") && lt(sub(c, 2, 2), b"Y") {
                if eq(sub(g, 1, 2), b"GF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"CP") {
                if (eq(sub(g, 1, 2), b"FH") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FG") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"GH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"FI") && (eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"70"))) || (eq(sub(g, 1, 2), b"GG") && (eq(sub(g, 3, 4), b"09") || eq(sub(g, 3, 4), b"19"))) { f.gvalid = true; }
            } else if (eq(sub(c, 1, 2), b"C2") && eq(sub(g, 1, 4), b"RI39")) || (eq(sub(c, 1, 2), b"C3") && eq(sub(g, 1, 4), b"JN02")) {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"CN") {
                if (eq(sub(g, 1, 2), b"IM") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"IL") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79"))) { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 3), b"IM9") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"5")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"IL") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"3") || eq(sub(g, 4, 4), b":")) || (eq(sub(g, 4, 4), b"2") || eq(sub(g, 4, 4), b"6") || eq(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"39")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 2), b"C4") {
                if (eq(sub(g, 1, 3), b"KM6") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) || (eq(sub(g, 1, 3), b"KM7") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"C5") && eq(sub(g, 1, 2), b"IK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && eq(sub(g, 4, 4), b"3") {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"C6") && eq(sub(g, 1, 2), b"FL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7") {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"C8") || eq(sub(c, 1, 2), b"C9") {
                if (eq(sub(g, 1, 2), b"KG") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"LH") && eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
            } else if eq(sub(c, 2, 2), b"0") || eq(sub(c, 2, 2), b"1") || eq(sub(c, 2, 2), b"7") {
                f.wrongcall = true;
                return true;
            }
        } else if eq(sub(c, 1, 1), b"D") {
            if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"S") {
                if (eq(sub(g, 1, 2), b"JO") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"45"))) || (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"IB59") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"JO") && ((lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"35")) { f.gvalid = false; }
                }
            } else if eq(sub(c, 1, 2), b"DS") || eq(sub(c, 1, 2), b"DT") || (gt(sub(c, 2, 2), b"6") && lt(sub(c, 2, 2), b":")) {
                if (eq(sub(g, 1, 3), b"PM3") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 3), b"PM4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 4), b"PM24") || eq(sub(g, 1, 4), b"PM57") || eq(sub(g, 1, 4), b"GC07") { f.gvalid = true; }
            } else if gt(sub(c, 2, 2), b"T") && lt(sub(c, 2, 2), b"[") {
                if (eq(sub(g, 1, 2), b"PK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || (eq(sub(g, 1, 2), b"PJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"04"))) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OK") && eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 2), b"PL") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10"))) || eq(sub(g, 1, 4), b"OK70") || eq(sub(g, 1, 4), b"OK71") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"PK") && ((eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":"))) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"PJ") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"15")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"OJ") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 2), b"D0") || eq(sub(c, 1, 2), b"D1") {
                if eq(sub(g, 1, 3), b"KN8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":") || eq(sub(g, 1, 3), b"KN9") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":") || eq(sub(g, 1, 4), b"LN08") || eq(sub(g, 1, 4), b"LN09") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"D2") || eq(sub(c, 1, 2), b"D3") {
                if (eq(sub(g, 1, 2), b"JI") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"JH") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KH") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"28"))) || (eq(sub(g, 1, 3), b"KI0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 3), b"JH5") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"D4") && ((eq(sub(g, 1, 3), b"HK7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 3), b"HK8") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7"))) {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"D5") {
                if eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"67") && ne(sub(g, 3, 4), b"67") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"D6") && eq(sub(g, 1, 2), b"LH") && (eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"27")) {
                f.gvalid = true;
            }
        } else if eq(sub(c, 1, 1), b"E") {
            if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"I") {
                if (eq(sub(g, 1, 2), b"IN") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4") && ne(sub(g, 3, 4), b"50")) || (eq(sub(g, 1, 2), b"IM") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"85"))) || (eq(sub(g, 1, 2), b"JN") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"20"))) || (eq(sub(g, 1, 2), b"JM") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09") || eq(sub(g, 3, 4), b"19") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"IL") && ((eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || (eq(sub(g, 4, 4), b"8") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || eq(sub(g, 3, 4), b"39"))) { f.gvalid = true; }
            } else if gt(sub(c, 2, 2), b"L") && lt(sub(c, 2, 2), b"P") {
                if (eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"52") || eq(sub(g, 3, 4), b"62"))) || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"KO") && (eq(sub(g, 3, 4), b"81") || eq(sub(g, 3, 4), b"91")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"KN") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"95") || eq(sub(g, 3, 4), b"96")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 2), b"EI") || eq(sub(c, 1, 2), b"EJ") {
                if eq(sub(g, 1, 2), b"IO") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"6") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"E2") {
                if (eq(sub(g, 1, 2), b"OK") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"NK") && ((gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"87") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89"))) || (eq(sub(g, 1, 2), b"NJ") && ((eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89"))) || (eq(sub(g, 1, 2), b"OJ") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"16"))) || eq(sub(g, 1, 4), b"NL90") || eq(sub(g, 1, 4), b"OL00") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"E3") {
                if (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"LK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"E4") && (eq(sub(g, 1, 4), b"KM71") || eq(sub(g, 1, 4), b"KM72")) {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"E5") {
                if (eq(sub(g, 1, 2), b"BH") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10"))) || (eq(sub(g, 1, 2), b"BG") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"19"))) || (eq(sub(g, 1, 2), b"AH") && (eq(sub(g, 3, 4), b"78") || eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"81") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"99"))) || (eq(sub(g, 1, 2), b"BI") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"11"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"E6") {
                if eq(sub(g, 1, 4), b"AH50") || eq(sub(g, 1, 4), b"AH51") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"E7") {
                if eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6") && ne(sub(g, 3, 4), b"72") && ne(sub(g, 3, 4), b"73") { f.gvalid = true; }
            } else if gt(sub(c, 2, 2), b"T") && lt(sub(c, 2, 2), b"X") {
                if eq(sub(g, 1, 2), b"KO") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"ES") {
                if (eq(sub(g, 1, 2), b"KO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"KO49") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"ER") {
                if (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 4), b"KN56") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"EK") {
                if (eq(sub(g, 1, 2), b"LN") && (eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"21"))) || (eq(sub(g, 1, 2), b"LM") && (eq(sub(g, 3, 4), b"29") || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"38"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"EX") {
                if (eq(sub(g, 1, 2), b"MN") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"EY") {
                if (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"MN") && (eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"50"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"EZ") {
                if (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"MN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"EL") {
                if eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"67") && ne(sub(g, 3, 4), b"67") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"EP") || eq(sub(c, 1, 2), b"EQ") {
                if (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"LL") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"85") || eq(sub(g, 3, 4), b"95"))) || (eq(sub(g, 1, 2), b"ML") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"15") && ne(sub(g, 3, 4), b"19")) || (eq(sub(g, 1, 2), b"MM") && eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"LM") && ((eq(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"78") && ne(sub(g, 3, 4), b"88"))) { f.gvalid = false; }
                }
            } else if eq(sub(c, 1, 2), b"ET") {
                if (eq(sub(g, 1, 2), b"LJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"KJ") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) { f.gvalid = true; }
            } else if eq(sub(c, 2, 2), b"0") || eq(sub(c, 2, 2), b"1") || eq(sub(c, 2, 2), b"8") || eq(sub(c, 2, 2), b"9") {
                f.wrongcall = true;
                return true;
            }
        } else if eq(sub(c, 1, 1), b"F") {
            if (gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":")) || (gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"G")) || eq(sub(c, 2, 2), b"I") || eq(sub(c, 2, 2), b"L") || eq(sub(c, 2, 2), b"N") || eq(sub(c, 2, 2), b"Q") || eq(sub(c, 2, 2), b"U") || eq(sub(c, 2, 2), b"V") || eq(sub(c, 2, 2), b"X") || eq(sub(c, 2, 2), b"Z") {
                if (eq(sub(g, 1, 2), b"IN") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"IN7") && (eq(sub(g, 4, 4), b"7") || eq(sub(g, 4, 4), b"8"))) || (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"JN4") && (eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || eq(sub(g, 1, 4), b"JN02") || eq(sub(g, 1, 4), b"JN12") || eq(sub(g, 1, 4), b"IN92") || (eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"11"))) { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"IN") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) { f.gvalid = false; }
                }
            } else if eq(sub(c, 1, 2), b"FM") && eq(sub(g, 1, 4), b"FK94") {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"FG") {
                if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"96") || eq(sub(g, 3, 4), b"95")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FP") {
                if eq(sub(g, 1, 2), b"GN") && (eq(sub(g, 3, 4), b"16") || eq(sub(g, 3, 4), b"17")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FS") && eq(sub(g, 1, 4), b"FK88") {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"FJ") && eq(sub(g, 1, 4), b"FK87") {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"FY") {
                if eq(sub(g, 1, 2), b"GJ") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FR") {
                if eq(sub(g, 1, 2), b"LG") && (eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"78")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FK") {
                if (eq(sub(g, 1, 2), b"RG") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"RH") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"RG") && (eq(sub(g, 3, 4), b"19") || eq(sub(g, 3, 4), b"57") || eq(sub(g, 3, 4), b"67"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FK") {
                if (eq(sub(g, 1, 2), b"QH") && (eq(sub(g, 3, 4), b"90") || eq(sub(g, 3, 4), b"91"))) || eq(sub(g, 1, 4), b"QG98") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FH") {
                if eq(sub(g, 1, 2), b"LH") && (eq(sub(g, 3, 4), b"27") || eq(sub(g, 3, 4), b"26")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FT") {
                if (eq(sub(g, 1, 2), b"LH") && (eq(sub(g, 3, 4), b"38") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"74"))) || (eq(sub(g, 1, 2), b"LE") && (eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"54") || eq(sub(g, 3, 4), b"63"))) || (eq(sub(g, 1, 2), b"ME") && (eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"41") || eq(sub(g, 3, 4), b"50"))) || (eq(sub(g, 1, 2), b"MF") && (eq(sub(g, 3, 4), b"81") || eq(sub(g, 3, 4), b"82"))) || eq(sub(g, 1, 4), b"LG07") || eq(sub(g, 1, 4), b"KG98") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FO") {
                if (eq(sub(g, 1, 2), b"BH") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"BG") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"BI9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"CI") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01"))) || eq(sub(g, 1, 4), b"CH09") || eq(sub(g, 1, 4), b"DK50") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"FW") {
                if eq(sub(g, 1, 2), b"AH") && (eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"16")) { f.gvalid = true; }
            }
        } else if eq(sub(c, 1, 1), b"G") {
            if (eq(sub(g, 1, 2), b"IO") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 3), b"JO0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 1, 4), b"IP90") || eq(sub(g, 1, 4), b"IN69") || eq(sub(g, 1, 4), b"IN89") || eq(sub(g, 1, 4), b"IO37") { f.gvalid = true; }
            if f.gvalid && eq(sub(g, 1, 2), b"IO") {
                if (eq(sub(g, 1, 3), b"IO5") && ne(sub(g, 4, 4), b"4") && ne(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 3), b"IO6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"96") || eq(sub(g, 3, 4), b"98") { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 1), b"H") {
            if eq(sub(c, 1, 2), b"HA") || eq(sub(c, 1, 2), b"HG") {
                if (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"16")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HB") || eq(sub(c, 1, 2), b"HE") {
                if ((eq(sub(g, 1, 3), b"JN3") || eq(sub(g, 1, 3), b"JN4")) && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 1, 4), b"JN56") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HS") {
                if (eq(sub(g, 1, 2), b"OK") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"NK") && ((gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"87") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89"))) || (eq(sub(g, 1, 2), b"NJ") && ((eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89"))) || (eq(sub(g, 1, 2), b"OJ") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"16"))) || eq(sub(g, 1, 4), b"NL90") || eq(sub(g, 1, 4), b"OL00") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"H2") {
                if (eq(sub(g, 1, 3), b"KM6") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) || (eq(sub(g, 1, 3), b"KM7") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HF") {
                if (eq(sub(g, 1, 2), b"JO") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"20"))) || eq(sub(g, 1, 4), b"KN09") || eq(sub(g, 1, 4), b"KN19") || eq(sub(g, 1, 4), b"KO20") || eq(sub(g, 1, 4), b"JN99") || eq(sub(g, 1, 4), b"JN89") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"H6") || eq(sub(c, 1, 2), b"H7") || eq(sub(c, 1, 2), b"HT") {
                if eq(sub(g, 1, 2), b"EK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HC") || eq(sub(c, 1, 2), b"HD") {
                if (eq(sub(g, 1, 2), b"FI") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"06"))) || (eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"0")) || eq(sub(g, 3, 4), b"01"))) || (eq(sub(g, 1, 2), b"EI") && ((eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"58"))) || (eq(sub(g, 1, 2), b"EJ") && (eq(sub(g, 3, 4), b"90") || eq(sub(g, 3, 4), b"40"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HH") {
                if (eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"38") || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49"))) || eq(sub(g, 1, 4), b"FL30") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HI") {
                if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"47")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HL") {
                if (eq(sub(g, 1, 3), b"PM3") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 3), b"PM4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 4), b"PM24") || eq(sub(g, 1, 4), b"PM57") || eq(sub(g, 1, 4), b"GC07") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HK") || eq(sub(c, 1, 2), b"HJ") {
                if (eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"7")) || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"02") || eq(sub(g, 3, 4), b"03"))) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3") && ne(sub(g, 3, 4), b"22")) || (eq(sub(g, 1, 2), b"FI") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"55") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"56")) && ne(sub(g, 3, 4), b"27")) || (eq(sub(g, 1, 2), b"EJ") && (eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"94"))) || eq(sub(g, 1, 4), b"EK92") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HZ") {
                if (eq(sub(g, 1, 2), b"LL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8")) || (eq(sub(g, 1, 2), b"KL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"LK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10"))) || (eq(sub(g, 1, 2), b"KM") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KM92") { f.gvalid = true; }
            } else if eq(sub(c, 2, 2), b"P") || eq(sub(c, 2, 2), b"3") || eq(sub(c, 2, 2), b"8") || eq(sub(c, 2, 2), b"9") || eq(sub(c, 2, 2), b"O") {
                if (eq(sub(g, 1, 2), b"FJ") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09") || eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"18"))) || (eq(sub(g, 1, 2), b"EJ") && (eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"97") || eq(sub(g, 3, 4), b"98") || eq(sub(g, 3, 4), b"99"))) { f.gvalid = true; }
            } else if eq(sub(c, 2, 2), b"R") || eq(sub(c, 2, 2), b"Q") {
                if eq(sub(g, 1, 2), b"EK") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7") && ne(sub(g, 3, 4), b"56") && ne(sub(g, 3, 4), b"83") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HN") {
                if (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"KM") && (eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"93"))) || (eq(sub(g, 1, 2), b"LL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"HU") {
                if eq(sub(g, 1, 2), b"EK") && (eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"54") || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"43")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"H5") {
                if (eq(sub(g, 1, 2), b"KG") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")))) || (eq(sub(g, 1, 2), b"KF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JF") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"85")) || (eq(sub(g, 1, 2), b"JG") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KE83") || eq(sub(g, 1, 4), b"KE93") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"KF") && ((eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"57")) { f.gvalid = false; }
                }
            } else if gt(sub(c, 2, 2), b"V") && lt(sub(c, 2, 2), b"Z") {
                if (eq(sub(g, 1, 2), b"IN") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"IN7") && (eq(sub(g, 4, 4), b"7") || eq(sub(g, 4, 4), b"8"))) || (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"JN4") && (eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || eq(sub(g, 1, 4), b"JN02") || eq(sub(g, 1, 4), b"JN12") || eq(sub(g, 1, 4), b"IN92") || (eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"11"))) { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"IN") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) { f.gvalid = false; }
                }
            } else if eq(sub(c, 1, 2), b"H1") {
                f.wrongcall = true;
                return true;
            }
        } else if eq(sub(c, 1, 1), b"I") {
            if (eq(sub(g, 1, 2), b"JN") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"41"))) || (eq(sub(g, 1, 2), b"JM") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"65") || eq(sub(g, 3, 4), b"56"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"JM") && (eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"96") || eq(sub(g, 3, 4), b"97") || eq(sub(g, 3, 4), b"98")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"JN") && ((eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"50")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 1), b"J") {
            if eq(sub(c, 1, 2), b"JA") || (gt(sub(c, 2, 2), b"D") && lt(sub(c, 2, 2), b"T")) {
                if (eq(sub(g, 1, 2), b"PM") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 3), b"QM0") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"QN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 3), b"PN9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"PL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"QM19") || eq(sub(g, 1, 4), b"KC90") { f.gvalid = true; }
                if f.gvalid {
                    if eq(sub(g, 1, 2), b"PM") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3") && ne(sub(g, 3, 4), b"62")) || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"55") || eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"PL") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"38") || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"57") || eq(sub(g, 3, 4), b"59")) {
                        f.gvalid = false;
                    } else if eq(sub(g, 1, 2), b"QN") && ((eq(sub(g, 3, 3), b"2") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"5"))) || eq(sub(g, 3, 4), b"10")) {
                        f.gvalid = false;
                    }
                }
            } else if eq(sub(c, 1, 2), b"J4") {
                if (eq(sub(g, 1, 2), b"KM") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"14")) && ne(sub(g, 3, 4), b"05")) || eq(sub(g, 1, 4), b"JM99") || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"J2") {
                if eq(sub(g, 1, 2), b"LK") && (eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"12")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"J3") {
                if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"91")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"J5") {
                if eq(sub(g, 1, 2), b"IK") && (eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"20")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"J6") {
                if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"94")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"J7") && eq(sub(g, 1, 4), b"FK95") {
                f.gvalid = true;
            } else if eq(sub(c, 1, 2), b"J8") {
                if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"92")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 3), b"JD1") {
                if eq(sub(g, 1, 2), b"QL") && (eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"16") || eq(sub(g, 3, 4), b"17")) { f.gvalid = true; }
            } else if eq(sub(c, 2, 2), b"T") || eq(sub(c, 2, 2), b"U") || eq(sub(c, 2, 2), b"V") {
                if (eq(sub(g, 1, 2), b"ON") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"NN") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"NO") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"OO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"OO") && (eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"70"))) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"JW") {
                if (eq(sub(g, 1, 2), b"JQ") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KQ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JR") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && eq(sub(g, 4, 4), b"0")) || (eq(sub(g, 1, 2), b"KR") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && eq(sub(g, 4, 4), b"0")) || eq(sub(g, 1, 4), b"JQ94") { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"JX") {
                if eq(sub(g, 1, 2), b"IQ") && (eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"61")) { f.gvalid = true; }
            } else if eq(sub(c, 1, 2), b"JY") {
                if (eq(sub(g, 1, 2), b"KM") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"KL") && (eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"89"))) { f.gvalid = true; }
            } else if eq(sub(c, 2, 2), b"B") || eq(sub(c, 2, 2), b"C") || eq(sub(c, 2, 2), b"1") || eq(sub(c, 2, 2), b"9") || eq(sub(c, 2, 2), b"Z") || (eq(sub(c, 1, 2), b"JD") && gt(sub(c, 3, 3), b"1") && lt(sub(c, 3, 3), b":")) {
                f.wrongcall = true;
                return true;
            }
        } else if eq(sub(c, 1, 1), b"K") {
            if gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":") && gt(sub(c, 3, 3), b"/") && lt(sub(c, 3, 3), b":") { return true; }
            if (eq(sub(g, 1, 2), b"FM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"DL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"EM") || eq(sub(g, 1, 2), b"DM") || eq(sub(g, 1, 2), b"EN") || (eq(sub(g, 1, 2), b"CM") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"DN") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 2), b"BP") || (eq(sub(g, 1, 2), b"BO") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AP") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 1, 4), b"FK28") || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"BL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"BK") && (eq(sub(g, 3, 4), b"19") || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"AL") && (eq(sub(g, 3, 4), b"91") || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"27") || eq(sub(g, 3, 4), b"36") || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"93"))) || (eq(sub(g, 1, 3), b"QK2") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"RO") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 3, 4), b"63"))) || eq(sub(g, 1, 4), b"AP30") || eq(sub(g, 1, 4), b"QK36") || eq(sub(g, 1, 4), b"QL20") || eq(sub(g, 1, 4), b"RK39") || eq(sub(g, 1, 4), b"AK56") || eq(sub(g, 1, 4), b"AH48") || eq(sub(g, 1, 4), b"JA00") || eq(sub(g, 1, 4), b"RB32") || eq(sub(g, 1, 4), b"CP00") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"FM") && (eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"24")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"28") && ne(sub(g, 3, 4), b"58")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"14")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DL") && (eq(sub(g, 3, 4), b"78") || eq(sub(g, 3, 4), b"88")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EN") && ((eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || ((eq(sub(g, 3, 3), b"8") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && eq(sub(g, 4, 4), b"9") && ne(sub(g, 3, 3), b"2"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"CM") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FN") && ((eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"52")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"CO") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"BO") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"14") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"35")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"AP") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"AO") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":") && ne(sub(g, 4, 4), b"6")) || ((eq(sub(g, 3, 3), b"6") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"97") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"87") || eq(sub(g, 3, 4), b"31")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"BL") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"RO") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"71")) {
                    f.gvalid = false;
                }
            }
        }
        if block_2(c, g, f) { return true; }
        if block_4(c, g, f) { return true; }
    }
    return true;
    false
}

#[allow(clippy::all, clippy::pedantic, unused_parens, unreachable_code, unused_variables)]
fn block_2(c: &[u8; 12], g: &[u8; 12], f: &mut Flags) -> bool {
    if eq(sub(c, 1, 1), b"L") {
        if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"O") {
            if (eq(sub(g, 1, 2), b"JO") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"37"))) || (eq(sub(g, 1, 2), b"JP") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"KP") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KQ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"JQ90") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"JP") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"97")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KP") && eq(sub(g, 4, 4), b"8") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KQ") && (eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"51")) {
                    f.gvalid = false;
                }
            }
        } else if gt(sub(c, 2, 2), b"N") && lt(sub(c, 2, 2), b"X") {
            if (eq(sub(g, 1, 2), b"GF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FF") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FG") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"GG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"FE") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"9")) || (eq(sub(g, 1, 2), b"FD") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"GC07") || eq(sub(g, 1, 4), b"GC16") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"GF") && (eq(sub(g, 3, 4), b"16") || eq(sub(g, 3, 4), b"17")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FG") && (eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"57")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GG") && (eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"35")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FE") && ((eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"70") || eq(sub(g, 3, 4), b"73")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FD") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":"))) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"LX") {
            if (eq(sub(g, 1, 2), b"JN") && (eq(sub(g, 3, 4), b"29") || eq(sub(g, 3, 4), b"39"))) || (eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"30"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"LY") {
            if eq(sub(g, 1, 2), b"KO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"LZ") {
            if eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"5") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"0") && lt(sub(c, 2, 2), b":") {
            if (eq(sub(g, 1, 2), b"GF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FF") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FG") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"GG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"FE") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"9")) || (eq(sub(g, 1, 2), b"FD") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"GC07") || eq(sub(g, 1, 4), b"GC16") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"GF") && (eq(sub(g, 3, 4), b"16") || eq(sub(g, 3, 4), b"17")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FG") && (eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"57")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GG") && (eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"35")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FE") && ((eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"70") || eq(sub(g, 3, 4), b"73")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FD") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":"))) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"L0") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"M") {
        if (eq(sub(g, 1, 2), b"IO") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 3), b"JO0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 1, 4), b"IP90") || eq(sub(g, 1, 4), b"IN69") || eq(sub(g, 1, 4), b"IN89") || eq(sub(g, 1, 4), b"IO37") { f.gvalid = true; }
        if f.gvalid && eq(sub(g, 1, 2), b"IO") {
            if (eq(sub(g, 1, 3), b"IO5") && ne(sub(g, 4, 4), b"4") && ne(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 3), b"IO6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"96") || eq(sub(g, 3, 4), b"98") { f.gvalid = false; }
        }
    } else if eq(sub(c, 1, 1), b"N") {
        if gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":") && gt(sub(c, 3, 3), b"/") && lt(sub(c, 3, 3), b":") { return true; }
        if (eq(sub(g, 1, 2), b"FM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"DL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"EM") || eq(sub(g, 1, 2), b"DM") || eq(sub(g, 1, 2), b"EN") || (eq(sub(g, 1, 2), b"CM") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"DN") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 2), b"BP") || (eq(sub(g, 1, 2), b"BO") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AP") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 1, 4), b"FK28") || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"BL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"BK") && (eq(sub(g, 3, 4), b"19") || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"AL") && (eq(sub(g, 3, 4), b"91") || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"27") || eq(sub(g, 3, 4), b"36") || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"93"))) || (eq(sub(g, 1, 3), b"QK2") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"RO") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 3, 4), b"63"))) || eq(sub(g, 1, 4), b"AP30") || eq(sub(g, 1, 4), b"QK36") || eq(sub(g, 1, 4), b"QL20") || eq(sub(g, 1, 4), b"RK39") || eq(sub(g, 1, 4), b"AK56") || eq(sub(g, 1, 4), b"AH48") || eq(sub(g, 1, 4), b"JA00") || eq(sub(g, 1, 4), b"RB32") || eq(sub(g, 1, 4), b"CP00") { f.gvalid = true; }
        if f.gvalid {
            if eq(sub(g, 1, 2), b"FM") && (eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"24")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"EL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"28") && ne(sub(g, 3, 4), b"58")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"14")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"DL") && (eq(sub(g, 3, 4), b"78") || eq(sub(g, 3, 4), b"88")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"DM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"EN") && ((eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || ((eq(sub(g, 3, 3), b"8") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && eq(sub(g, 4, 4), b"9") && ne(sub(g, 3, 3), b"2"))) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"CM") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"FN") && ((eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"52")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"CO") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"39")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"BO") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"14") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"35")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"AP") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"AO") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":") && ne(sub(g, 4, 4), b"6")) || ((eq(sub(g, 3, 3), b"6") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"97") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"87") || eq(sub(g, 3, 4), b"31")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"BL") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"RO") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"71")) {
                f.gvalid = false;
            }
        }
    } else if eq(sub(c, 1, 1), b"O") {
        if gt(sub(c, 2, 2), b"E") && lt(sub(c, 2, 2), b"K") {
            if (eq(sub(g, 1, 2), b"KP") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 1, 4), b"JP90") || eq(sub(g, 1, 4), b"KO09") || eq(sub(g, 1, 4), b"KO19") || eq(sub(g, 1, 4), b"KQ30") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"M") && lt(sub(c, 2, 2), b"U") {
            if (eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"30"))) || eq(sub(g, 1, 4), b"JN29") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"OE") {
            if eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"9") { f.gvalid = true; }
        } else if eq(sub(c, 2, 2), b"K") || eq(sub(c, 2, 2), b"L") {
            if (eq(sub(g, 1, 2), b"JO") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && eq(sub(g, 4, 4), b"0")) || (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"JO71") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"OM") {
            if (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
        } else if eq(sub(c, 2, 2), b"Z") || eq(sub(c, 2, 2), b"U") || eq(sub(c, 2, 2), b"V") {
            if eq(sub(g, 1, 2), b"JO") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"76")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"OD") {
            if eq(sub(g, 1, 2), b"KM") && (eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"74") || eq(sub(g, 3, 4), b"84")) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"D") {
            if (eq(sub(g, 1, 2), b"FH") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FI") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 3), b"EI9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"OX") || eq(sub(c, 1, 2), b"XP") {
            if (eq(sub(g, 1, 2), b"GP") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"HP") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"HQ90") || (eq(sub(g, 1, 2), b"GQ") && (eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"41") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"14") || eq(sub(g, 3, 4), b"12"))) || (eq(sub(g, 1, 2), b"FQ") && (eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"57") || eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"38"))) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"P") {
        if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"J") {
            if ((eq(sub(g, 1, 3), b"JO2") || eq(sub(g, 1, 3), b"JO3")) && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 1, 4), b"JO11") { f.gvalid = true; }
        } else if eq(sub(c, 1, 1), b"P") && gt(sub(c, 2, 2), b"O") && lt(sub(c, 2, 2), b"Z") {
            if (eq(sub(g, 1, 2), b"GG") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"HI") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6") && ne(sub(g, 3, 4), b"25")) || eq(sub(g, 3, 4), b"06") || eq(sub(g, 3, 4), b"07") || eq(sub(g, 3, 4), b"36"))) || (eq(sub(g, 1, 2), b"HH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2")) || (eq(sub(g, 1, 2), b"GH") && ne(sub(g, 3, 4), b"01")) || eq(sub(g, 1, 2), b"GI") || (eq(sub(g, 1, 2), b"GF") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"36"))) || (eq(sub(g, 1, 2), b"GJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"44"))) || (eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"95"))) || (eq(sub(g, 1, 2), b"FI") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FH") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"94") || eq(sub(g, 3, 4), b"95"))) || eq(sub(g, 1, 4), b"HG59") || eq(sub(g, 1, 4), b"HJ50") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"GG") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6") && ne(sub(g, 3, 4), b"64") && ne(sub(g, 3, 4), b"65")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"HH") && (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GI") && ((gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GF") && (eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"27")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GJ") && (eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"52") || eq(sub(g, 3, 4), b"53")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FJ") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FI") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FH") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"68")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"P2") {
            if (eq(sub(g, 1, 2), b"QI") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"QH") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"P3") {
            if (eq(sub(g, 1, 3), b"KM6") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) || (eq(sub(g, 1, 3), b"KM7") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"P4") && (eq(sub(g, 1, 4), b"FK42") || eq(sub(g, 1, 4), b"FK52")) {
            f.gvalid = true;
        } else if gt(sub(c, 2, 2), b"4") && lt(sub(c, 2, 2), b":") {
            if (eq(sub(g, 1, 2), b"PM") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"PN") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"PJ") && ((eq(sub(c, 3, 3), b"2") || eq(sub(c, 3, 3), b"4")) && eq(sub(g, 1, 4), b"FK52")) || ((eq(sub(c, 3, 3), b"5") || eq(sub(c, 3, 3), b"6")) && eq(sub(g, 1, 4), b"FK87")) || ((eq(sub(c, 3, 3), b"7") || eq(sub(c, 3, 3), b"8") || eq(sub(c, 3, 3), b"0")) && eq(sub(g, 1, 4), b"FK88")) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"PZ") {
            if eq(sub(g, 1, 2), b"GJ") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"J") && lt(sub(c, 2, 2), b"P") {
            if (eq(sub(g, 1, 2), b"NJ") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"NI") && gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OI") && !(gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) && !(gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && eq(sub(g, 4, 4), b"0"))) || (eq(sub(g, 1, 2), b"PI") && !(gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && eq(sub(g, 4, 4), b"0"))) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"PJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"QI") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"PH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"PJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NJ") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"7") && ((gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"3"))) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OJ") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"04")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OI") && ((eq(sub(g, 4, 4), b"1") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PI") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"83") || eq(sub(g, 3, 4), b"25") || eq(sub(g, 3, 4), b"35") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"29")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"P1") || eq(sub(c, 1, 2), b"P0") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"R") {
        if (eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"6")))) || eq(sub(g, 1, 2), b"LO") || eq(sub(g, 1, 2), b"MO") || eq(sub(g, 1, 2), b"NO") || (eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KP") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"30"))) || eq(sub(g, 1, 2), b"NP") || eq(sub(g, 1, 2), b"OO") || eq(sub(g, 1, 2), b"PO") || eq(sub(g, 1, 2), b"LP") || eq(sub(g, 1, 2), b"MP") || (eq(sub(g, 1, 2), b"PN") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"52") || eq(sub(g, 3, 4), b"62"))) || eq(sub(g, 1, 4), b"MQ60") || (eq(sub(g, 1, 2), b"QN") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"QO") && (eq(sub(g, 3, 3), b"0") || eq(sub(g, 3, 3), b"1") || eq(sub(g, 3, 3), b"8") || eq(sub(g, 3, 3), b"9") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"59"))) || eq(sub(g, 1, 4), b"JO94") || eq(sub(g, 1, 2), b"OP") || eq(sub(g, 1, 2), b"PP") || eq(sub(g, 1, 4), b"JB59") || eq(sub(g, 1, 4), b"NQ03") || (eq(sub(g, 1, 2), b"RP") && (eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"84"))) { f.gvalid = true; }
        if f.gvalid {
            if eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"35") || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"61")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"LO") && (eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"60")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"LN") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"1")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"48")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"MO") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && eq(sub(g, 4, 4), b"3"))) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"KN") && (gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && ((gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 4, 4), b"3"))) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"KP") && (eq(sub(g, 3, 4), b"42") || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"99")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"OO") && eq(sub(g, 3, 4), b"00") {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"PO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"0")) || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"21")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"LP") && eq(sub(g, 4, 4), b"9") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"9") && ne(sub(g, 3, 3), b"4") && ne(sub(g, 3, 3), b"5") {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"PN") && (eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"83") || eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"94") || eq(sub(g, 3, 4), b"95")) {
                f.gvalid = false;
            }
        }
    } else if eq(sub(c, 1, 1), b"S") {
        if eq(sub(c, 1, 1), b"S") && gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"N") {
            if (eq(sub(g, 1, 2), b"JO") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JP") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"KP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"25"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"85") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"JP") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"99")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KP") && (eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"14")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 1), b"S") && gt(sub(c, 2, 2), b"M") && lt(sub(c, 2, 2), b"S") {
            if (eq(sub(g, 1, 2), b"JO") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"20"))) || eq(sub(g, 1, 4), b"KN09") || eq(sub(g, 1, 4), b"KN19") || eq(sub(g, 1, 4), b"KO20") || eq(sub(g, 1, 4), b"JN99") || eq(sub(g, 1, 4), b"JN89") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"U") && lt(sub(c, 2, 2), b"[") {
            if (eq(sub(g, 1, 2), b"KM") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"14")) && ne(sub(g, 3, 4), b"05")) || eq(sub(g, 1, 4), b"JM99") || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"S5") {
            if eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"7") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9V") {
            if eq(sub(g, 1, 2), b"OJ") && (eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"21")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"S0") {
            if eq(sub(g, 1, 2), b"IL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"S2") || eq(sub(c, 1, 2), b"S3") {
            if eq(sub(g, 1, 2), b"NL") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"S4") || eq(sub(c, 1, 2), b"S8") {
            if (eq(sub(g, 1, 2), b"KG") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")))) || (eq(sub(g, 1, 2), b"KF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JF") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"85")) || (eq(sub(g, 1, 2), b"JG") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KE83") || eq(sub(g, 1, 4), b"KE93") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"KF") && ((eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"57")) { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 2), b"S7") {
            if (eq(sub(g, 1, 2), b"LI") && (eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"74") || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"82") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"30"))) || (eq(sub(g, 1, 2), b"LH") && (eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"39"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"S9") && (eq(sub(g, 1, 4), b"JJ30") || eq(sub(g, 1, 4), b"JJ31")) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"ST") {
            if (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"KL") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KJ79") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"S1") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"T") {
        if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"D") {
            if (eq(sub(g, 1, 2), b"KM") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"85"))) || (eq(sub(g, 1, 2), b"KN") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"72"))) || (eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"16")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TF") {
            if (eq(sub(g, 1, 2), b"HP") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 2), b"IP") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TK") {
            if eq(sub(g, 1, 3), b"JN4") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TR") {
            if eq(sub(c, 1, 5), b"TR8CA") || (eq(sub(g, 1, 2), b"JJ") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"52") || eq(sub(g, 3, 4), b"62"))) || (eq(sub(g, 1, 2), b"JI") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"56"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TS") {
            if eq(sub(g, 1, 2), b"JM") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TG") || eq(sub(c, 1, 2), b"TD") {
            if eq(sub(g, 1, 2), b"EK") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"8") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TI") || eq(sub(c, 1, 2), b"TE") {
            if (eq(sub(g, 1, 2), b"EK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"EJ") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"EJ65") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"T6") {
            if (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"ML") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && eq(sub(g, 3, 4), b"9")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"T7") && eq(sub(g, 1, 4), b"JN63") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"T8") {
            if eq(sub(g, 1, 2), b"PJ") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"9") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"T4") {
            if (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || eq(sub(g, 1, 4), b"FK19") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TO") {
            if (eq(sub(g, 1, 3), b"FK9") && (eq(sub(g, 4, 4), b"6") || eq(sub(g, 4, 4), b"5") || eq(sub(g, 4, 4), b"4"))) || (eq(sub(g, 1, 3), b"FK8") && (eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"7"))) || (eq(sub(g, 1, 2), b"LG") && (eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"78"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"T2") {
            if (eq(sub(g, 1, 2), b"RI") && (eq(sub(g, 3, 4), b"91") || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"82") || eq(sub(g, 3, 4), b"83") || eq(sub(g, 3, 4), b"84"))) || eq(sub(g, 1, 4), b"RH99") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"T3") {
            if eq(sub(g, 1, 1), b"R") {
                if (eq(sub(g, 1, 3), b"RJ6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 3), b"RI7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"RI49") || eq(sub(g, 1, 4), b"RI87") || eq(sub(g, 1, 4), b"RI88") { f.gvalid = true; }
            } else if eq(sub(g, 1, 1), b"A") {
                if (eq(sub(g, 1, 2), b"AI") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 1, 4), b"AJ94") { f.gvalid = true; }
            } else if eq(sub(g, 1, 1), b"B") {
                if eq(sub(g, 1, 4), b"BJ03") || eq(sub(g, 1, 4), b"BJ11") || eq(sub(g, 1, 4), b"BJ12") || (eq(sub(g, 1, 3), b"BI2") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 1, 4), b"BH48") || eq(sub(g, 1, 4), b"BH49") || eq(sub(g, 1, 4), b"BI40") { f.gvalid = true; }
            }
        } else if eq(sub(c, 1, 2), b"TJ") {
            if (eq(sub(g, 1, 2), b"JJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TL") {
            if (eq(sub(g, 1, 2), b"JJ") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && eq(sub(g, 4, 4), b"0")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TN") {
            if (eq(sub(g, 1, 2), b"JI") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JJ") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TT") {
            if (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"JJ") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"KJ09") || (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2")) || (eq(sub(g, 1, 2), b"JL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"KL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TU") {
            if (eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"IK") && (eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"70") || eq(sub(g, 3, 4), b"50"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TY") {
            if (eq(sub(g, 1, 2), b"JJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TZ") {
            if (eq(sub(g, 1, 2), b"IK") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"IL") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"JL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TX") {
            if (eq(sub(g, 1, 2), b"QH") && (eq(sub(g, 3, 4), b"90") || eq(sub(g, 3, 4), b"91"))) || eq(sub(g, 1, 4), b"QG98") || (eq(sub(g, 1, 2), b"BH") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"BG") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"BI9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"CI") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01"))) || eq(sub(g, 1, 4), b"CH09") || eq(sub(g, 1, 4), b"DK50") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"TW") {
            if eq(sub(g, 1, 2), b"AH") && (eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"16")) { f.gvalid = true; }
        } else if eq(sub(c, 2, 2), b"H") || eq(sub(c, 2, 2), b"M") || eq(sub(c, 2, 2), b"P") || eq(sub(c, 2, 2), b"Q") || eq(sub(c, 2, 2), b"V") {
            if (eq(sub(g, 1, 2), b"IN") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"IN7") && (eq(sub(g, 4, 4), b"7") || eq(sub(g, 4, 4), b"8"))) || (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"JN4") && (eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || eq(sub(g, 1, 4), b"JN02") || eq(sub(g, 1, 4), b"JN12") || eq(sub(g, 1, 4), b"IN92") || (eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"11"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"IN") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 2), b"T1") || eq(sub(c, 1, 2), b"T0") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"U") {
        if (gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"J")) || (gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":")) {
            if (eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"6")))) || eq(sub(g, 1, 2), b"LO") || eq(sub(g, 1, 2), b"MO") || eq(sub(g, 1, 2), b"NO") || (eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KP") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"30"))) || eq(sub(g, 1, 2), b"NP") || eq(sub(g, 1, 2), b"OO") || eq(sub(g, 1, 2), b"PO") || eq(sub(g, 1, 2), b"LP") || eq(sub(g, 1, 2), b"MP") || (eq(sub(g, 1, 2), b"PN") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"52") || eq(sub(g, 3, 4), b"62"))) || eq(sub(g, 1, 4), b"MQ60") || (eq(sub(g, 1, 2), b"QN") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"QO") && (eq(sub(g, 3, 3), b"0") || eq(sub(g, 3, 3), b"1") || eq(sub(g, 3, 3), b"8") || eq(sub(g, 3, 3), b"9") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"59"))) || eq(sub(g, 1, 4), b"JO94") || eq(sub(g, 1, 2), b"OP") || eq(sub(g, 1, 2), b"PP") || eq(sub(g, 1, 4), b"JB59") || eq(sub(g, 1, 4), b"NQ03") || (eq(sub(g, 1, 2), b"RP") && (eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"84"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"35") || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"61")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"LO") && (eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"60")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"LN") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"1")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"48")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"MO") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && eq(sub(g, 4, 4), b"3"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KN") && (gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && ((gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 4, 4), b"3"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KP") && (eq(sub(g, 3, 4), b"42") || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"99")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OO") && eq(sub(g, 3, 4), b"00") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"0")) || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"21")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"LP") && eq(sub(g, 4, 4), b"9") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"9") && ne(sub(g, 3, 3), b"4") && ne(sub(g, 3, 3), b"5") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PN") && (eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"83") || eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"94") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                }
            }
        } else if gt(sub(c, 2, 2), b"Q") && lt(sub(c, 2, 2), b"[") {
            if (eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"52") || eq(sub(g, 3, 4), b"62"))) || (eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"KO") && (eq(sub(g, 3, 4), b"81") || eq(sub(g, 3, 4), b"91")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KN") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"95") || eq(sub(g, 3, 4), b"96")) {
                    f.gvalid = false;
                }
            }
        } else if gt(sub(c, 2, 2), b"M") && lt(sub(c, 2, 2), b"R") {
            if (eq(sub(g, 1, 2), b"MN") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"MO") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LO") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"NN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"NO") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10") || eq(sub(g, 3, 4), b"20"))) || (eq(sub(g, 1, 2), b"MO") && (eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"55"))) || (eq(sub(g, 1, 2), b"MN") && ((eq(sub(g, 4, 4), b"1") && (eq(sub(g, 3, 3), b"3") || eq(sub(g, 3, 3), b"4") || eq(sub(g, 3, 3), b"5"))) || eq(sub(g, 3, 4), b"40"))) || (eq(sub(g, 1, 3), b"NN0") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) { f.gvalid = true; }
        }
    } else if eq(sub(c, 1, 1), b"V") {
        if (gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"H")) || eq(sub(c, 1, 2), b"VO") || eq(sub(c, 1, 2), b"VX") || eq(sub(c, 1, 2), b"VY") {
            if (eq(sub(g, 1, 2), b"CN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FN") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"EN") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"CO") || (eq(sub(g, 1, 2), b"GN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"FO") || eq(sub(g, 1, 2), b"EO") || eq(sub(g, 1, 2), b"DO") || (eq(sub(g, 1, 2), b"GO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"DN") && gt(sub(g, 4, 4), b"8") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"CP") || eq(sub(g, 1, 2), b"DP") || eq(sub(g, 1, 2), b"EP") || (eq(sub(g, 1, 2), b"FP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"96"))) || (eq(sub(g, 1, 2), b"EQ") && (eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"96"))) || (eq(sub(g, 1, 2), b"FR") && (eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"82"))) || (eq(sub(g, 1, 2), b"FQ") && (eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"12"))) || eq(sub(g, 1, 4), b"GN03") || eq(sub(g, 1, 4), b"FN93") || eq(sub(g, 1, 4), b"ER60") || eq(sub(g, 1, 4), b"DQ10") || eq(sub(g, 1, 4), b"CQ71") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"EO") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"9")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"86")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"CO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"40")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EP") && ((eq(sub(g, 4, 4), b"0") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 4, 4), b"1") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9")) || (eq(sub(g, 4, 4), b"2") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"8"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"CN") && (eq(sub(g, 3, 4), b"68") || eq(sub(g, 3, 4), b"98")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"04")) {
                    f.gvalid = false;
                }
            }
        } else if (gt(sub(c, 2, 2), b"G") && lt(sub(c, 2, 2), b"O")) || eq(sub(c, 1, 2), b"VZ") {
            if (eq(sub(g, 1, 2), b"QG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 1, 2), b"QF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 1, 2), b"PF") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OF") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"QE") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"19"))) || (eq(sub(g, 1, 2), b"QH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"OG") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 1, 2), b"PG") || eq(sub(g, 1, 2), b"PH") || (eq(sub(c, 1, 3), b"VK0") && ((eq(sub(g, 1, 2), b"MD") && (eq(sub(g, 3, 4), b"66") || eq(sub(g, 3, 4), b"67"))) || (eq(sub(g, 1, 2), b"QD") && (eq(sub(g, 3, 4), b"95") || eq(sub(g, 3, 4), b"94"))))) || (eq(sub(c, 1, 3), b"VK9") && ((eq(sub(g, 1, 2), b"NH") && (eq(sub(g, 3, 4), b"87") || eq(sub(g, 3, 4), b"88"))) || eq(sub(g, 1, 4), b"QF98") || eq(sub(g, 1, 4), b"OH29") || (eq(sub(g, 1, 2), b"RG") && (eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31"))) || (eq(sub(g, 1, 2), b"QH") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"QF") && eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PF") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"7")) || eq(sub(g, 3, 4), b"57")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 4), b"OF74") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"QH") && ((eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"09")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OG") && (eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"69")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PH") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":") && ne(sub(g, 4, 4), b"5") && ne(sub(g, 4, 4), b"6")) || (gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"89")) {
                    f.gvalid = false;
                }
            }
        } else if gt(sub(c, 2, 2), b"S") && lt(sub(c, 2, 2), b"X") {
            if (eq(sub(g, 1, 2), b"MJ") && (eq(sub(g, 3, 4), b"99") || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"68"))) || (eq(sub(g, 1, 2), b"MK") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"52"))) || (eq(sub(g, 1, 2), b"NK") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"39"))) || (eq(sub(g, 1, 2), b"ML") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 1, 4), b"NM00") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"MK") && eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NK") && (eq(sub(g, 3, 4), b"26") || eq(sub(g, 3, 4), b"27")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"ML") && ((gt(sub(g, 3, 3), b"4") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"5") || eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NL") && ((eq(sub(g, 4, 4), b"9") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 4, 4), b"8") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"40")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"MM") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"62")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"VR") && (eq(sub(g, 1, 2), b"OL") && (eq(sub(g, 3, 4), b"72") || eq(sub(g, 3, 4), b"62"))) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"V2") {
            if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"97")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"V3") {
            if (eq(sub(g, 1, 2), b"EK") && eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"EK") && eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"9")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"V4") && eq(sub(g, 1, 4), b"FK87") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"V5") {
            if (eq(sub(g, 1, 2), b"JG") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JH") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"KG") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"V6") {
            if (eq(sub(g, 1, 2), b"PJ") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"QJ") && ((gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"73"))) || (eq(sub(g, 1, 2), b"RJ") && (eq(sub(g, 3, 4), b"06") || eq(sub(g, 3, 4), b"15"))) || eq(sub(g, 1, 4), b"PK90") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"V7") {
            if (eq(sub(g, 1, 2), b"RJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"RK") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"39"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"V8") {
            if eq(sub(g, 1, 2), b"OJ") && (eq(sub(g, 3, 4), b"74") || eq(sub(g, 3, 4), b"75")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 3), b"VP9") && eq(sub(g, 1, 4), b"FM72") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"VQ9") {
            if eq(sub(g, 1, 2), b"MI") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"V9") {
            if (eq(sub(g, 1, 2), b"KG") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")))) || (eq(sub(g, 1, 2), b"KF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JF") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"85")) || (eq(sub(g, 1, 2), b"JG") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KE83") || eq(sub(g, 1, 4), b"KE93") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"KF") && ((eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"57")) { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 3), b"VP2") && (eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"78"))) {
            f.gvalid = true;
        } else if (eq(sub(c, 1, 3), b"VP5") || eq(sub(c, 1, 3), b"VQ5")) && (eq(sub(g, 1, 2), b"FL") && (eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"41"))) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"VP6") && (eq(sub(g, 1, 2), b"CG") && (eq(sub(g, 3, 4), b"55") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"75"))) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"VP8") {
            if (eq(sub(g, 1, 2), b"FD") && gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 4), b"GD86") || (eq(sub(g, 1, 2), b"HD") && ((eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"25") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"16") || eq(sub(g, 3, 4), b"06")) || (eq(sub(g, 1, 3), b"HD6") && (gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4"))) || eq(sub(g, 1, 4), b"HD53"))) || eq(sub(g, 1, 4), b"GC69") || eq(sub(g, 1, 4), b"GC79") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"V1") || eq(sub(c, 1, 2), b"V0") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"W") {
        if gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":") && gt(sub(c, 3, 3), b"/") && lt(sub(c, 3, 3), b":") { return true; }
        if (eq(sub(g, 1, 2), b"FM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"DL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"EM") || eq(sub(g, 1, 2), b"DM") || eq(sub(g, 1, 2), b"EN") || (eq(sub(g, 1, 2), b"CM") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"DN") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"CO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 2), b"BP") || (eq(sub(g, 1, 2), b"BO") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AP") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 1, 4), b"FK28") || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"BL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"BK") && (eq(sub(g, 3, 4), b"19") || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"29"))) || (eq(sub(g, 1, 2), b"AL") && (eq(sub(g, 3, 4), b"91") || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"27") || eq(sub(g, 3, 4), b"36") || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"93"))) || (eq(sub(g, 1, 3), b"QK2") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"AO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"RO") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 3, 4), b"63"))) || eq(sub(g, 1, 4), b"AP30") || eq(sub(g, 1, 4), b"QK36") || eq(sub(g, 1, 4), b"QL20") || eq(sub(g, 1, 4), b"RK39") || eq(sub(g, 1, 4), b"AK56") || eq(sub(g, 1, 4), b"AH48") || eq(sub(g, 1, 4), b"JA00") || eq(sub(g, 1, 4), b"RB32") || eq(sub(g, 1, 4), b"CP00") { f.gvalid = true; }
        if f.gvalid {
            if eq(sub(g, 1, 2), b"FM") && (eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"24")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"EL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"28") && ne(sub(g, 3, 4), b"58")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"14")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"DL") && (eq(sub(g, 3, 4), b"78") || eq(sub(g, 3, 4), b"88")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"DM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"EN") && ((eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || ((eq(sub(g, 3, 3), b"8") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && eq(sub(g, 4, 4), b"9") && ne(sub(g, 3, 3), b"2"))) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"CM") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"FN") && ((eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"52")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"CO") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"39")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"BO") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"14") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"35")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"AP") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"AO") && ((eq(sub(g, 3, 3), b"4") && ((gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":") && ne(sub(g, 4, 4), b"6")) || ((eq(sub(g, 3, 3), b"6") || eq(sub(g, 3, 3), b"7")) && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"97") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"87") || eq(sub(g, 3, 4), b"31")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"BL") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22")) {
                f.gvalid = false;
            } else if eq(sub(g, 1, 2), b"RO") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"71")) {
                f.gvalid = false;
            }
        }
    } else if eq(sub(c, 1, 1), b"X") {
        if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"J") {
            if (eq(sub(g, 1, 2), b"DL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"EK") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"34"))) || (eq(sub(g, 1, 2), b"DM") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"DK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"DK") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"38"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"DL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"33") || eq(sub(g, 3, 4), b"60")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"62")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EK") && (eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"65") || eq(sub(g, 3, 4), b"66") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DM") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && eq(sub(g, 4, 4), b"2") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DK") && (eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"77") || eq(sub(g, 3, 4), b"86")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 1), b"X") && gt(sub(c, 2, 2), b"I") && lt(sub(c, 2, 2), b"P") {
            if (eq(sub(g, 1, 2), b"CN") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FN") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"EN") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"CO") || (eq(sub(g, 1, 2), b"GN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"FO") || eq(sub(g, 1, 2), b"EO") || eq(sub(g, 1, 2), b"DO") || (eq(sub(g, 1, 2), b"GO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"DN") && gt(sub(g, 4, 4), b"8") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"CP") || eq(sub(g, 1, 2), b"DP") || eq(sub(g, 1, 2), b"EP") || (eq(sub(g, 1, 2), b"FP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"96"))) || (eq(sub(g, 1, 2), b"EQ") && (eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"73") || eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"86") || eq(sub(g, 3, 4), b"96"))) || (eq(sub(g, 1, 2), b"FR") && (eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"82"))) || (eq(sub(g, 1, 2), b"FQ") && (eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"12"))) || eq(sub(g, 1, 4), b"GN03") || eq(sub(g, 1, 4), b"FN93") || eq(sub(g, 1, 4), b"ER60") || eq(sub(g, 1, 4), b"DQ10") || eq(sub(g, 1, 4), b"CQ71") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"EO") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 4, 4), b"7") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"9")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"86")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"CO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"40")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EP") && ((eq(sub(g, 4, 4), b"0") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 4, 4), b"1") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9")) || (eq(sub(g, 4, 4), b"2") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"8"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"CN") && (eq(sub(g, 3, 4), b"68") || eq(sub(g, 3, 4), b"98")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"04")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"XT") {
            if (eq(sub(g, 1, 2), b"IK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"IJ") && (eq(sub(g, 3, 4), b"79") || eq(sub(g, 3, 4), b"89"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"XU") {
            if eq(sub(g, 1, 2), b"OK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"XV") {
            if (eq(sub(g, 1, 2), b"OL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"OK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"OJ19") || eq(sub(g, 1, 4), b"OL41") || eq(sub(g, 1, 4), b"OL23") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"XW") {
            if (eq(sub(g, 1, 2), b"OK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 3), b"XX9") && eq(sub(g, 1, 4), b"OL62") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"XQ") || eq(sub(c, 1, 2), b"XR") {
            if (eq(sub(g, 1, 2), b"FF") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"FG") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")))) || (eq(sub(g, 1, 2), b"FE") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"FH") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"52"))) || (eq(sub(g, 1, 2), b"FD") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"85"))) || eq(sub(g, 1, 4), b"EF96") || eq(sub(g, 1, 4), b"FF06") || eq(sub(g, 1, 4), b"EG93") || eq(sub(g, 1, 4), b"FG03") || eq(sub(g, 1, 4), b"DG52") || eq(sub(g, 1, 4), b"DG73") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"FF") && eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FE") && (eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"41") || eq(sub(g, 3, 4), b"29")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"XY") || eq(sub(c, 1, 2), b"XZ") {
            if (eq(sub(g, 1, 2), b"NK") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"OL") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"XP") {
            if (eq(sub(g, 1, 2), b"GP") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"HP") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"HQ90") || (eq(sub(g, 1, 2), b"GQ") && (eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"41") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"14") || eq(sub(g, 3, 4), b"12"))) || (eq(sub(g, 1, 2), b"FQ") && (eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"57") || eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"38"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"XS") {
            if eq(sub(g, 1, 2), b"OL") || (eq(sub(g, 1, 2), b"PL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2")) || eq(sub(g, 1, 2), b"OM") || (eq(sub(g, 1, 2), b"PM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || (eq(sub(g, 1, 2), b"PN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8")) || (eq(sub(g, 1, 2), b"PO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"OK") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59"))) || eq(sub(g, 1, 2), b"ON") || eq(sub(g, 1, 4), b"OO90") || eq(sub(g, 1, 4), b"OO91") || eq(sub(g, 1, 2), b"NN") || eq(sub(g, 1, 2), b"NM") || (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"MN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"NL") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7") && ne(sub(g, 3, 4), b"83") && ne(sub(g, 3, 4), b"84") && ne(sub(g, 3, 4), b"85")) || eq(sub(g, 3, 4), b"77") || eq(sub(g, 3, 4), b"87")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"ON") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PN") && ((eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"MM") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OL") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && eq(sub(g, 4, 4), b"0")) || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"21")) {
                    f.gvalid = false;
                }
            }
        } else if gt(sub(c, 2, 2), b"/") && lt(sub(c, 2, 2), b":") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"Y") {
        if gt(sub(c, 2, 2), b"N") && lt(sub(c, 2, 2), b"S") {
            if eq(sub(g, 1, 2), b"KN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"9") { f.gvalid = true; }
        } else if eq(sub(c, 2, 2), b"U") || eq(sub(c, 2, 2), b"T") {
            if (eq(sub(g, 1, 3), b"JN9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 3), b"KN0") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 3), b"KN1") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"A") && lt(sub(c, 2, 2), b"I") {
            if (eq(sub(g, 1, 2), b"NJ") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"NI") && gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"OI") || eq(sub(g, 1, 2), b"PI") || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"PJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"QI") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"PH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"PJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NJ") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"7") && ((gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"3"))) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OJ") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"04")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OI") && ((eq(sub(g, 4, 4), b"1") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PI") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"83") || eq(sub(g, 3, 4), b"25") || eq(sub(g, 3, 4), b"35") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"29")) {
                    f.gvalid = false;
                }
            }
        } else if gt(sub(c, 2, 2), b"U") && lt(sub(c, 2, 2), b"Z") {
            if (eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"70"))) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 1, 4), b"FK85") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7") && ne(sub(g, 3, 4), b"57")) || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"93")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FJ") && (eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"72") || eq(sub(g, 3, 4), b"82") || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"91")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"YL") {
            if (eq(sub(g, 1, 2), b"KO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"KO") && (eq(sub(g, 3, 4), b"25") || eq(sub(g, 3, 4), b"35") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"28"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"YA") {
            if (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"ML") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && eq(sub(g, 3, 4), b"9")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"YI") {
            if (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"KM") && (eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"93"))) || (eq(sub(g, 1, 2), b"LL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"YJ") {
            if (eq(sub(g, 1, 2), b"RH") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || eq(sub(g, 1, 4), b"RG49") || eq(sub(g, 1, 4), b"RH50") || eq(sub(g, 1, 4), b"RH36") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"YK") {
            if (eq(sub(g, 1, 2), b"KM") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"YM") {
            if (eq(sub(g, 1, 2), b"KM") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"85"))) || (eq(sub(g, 1, 2), b"KN") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"72"))) || (eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"16")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"YN") {
            if eq(sub(g, 1, 2), b"EK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"YS") {
            if eq(sub(g, 1, 2), b"EK") && (eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"54") || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"43")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 1), b"Y") && gt(sub(c, 2, 2), b"1") && lt(sub(c, 2, 2), b":") {
            if (eq(sub(g, 1, 2), b"JO") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"45"))) || (eq(sub(g, 1, 2), b"JN") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"IB59") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"JO") && ((lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"35")) { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 2), b"Y1") || eq(sub(c, 1, 2), b"Y0") {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"Z") {
        if eq(sub(c, 1, 2), b"Z3") {
            if (eq(sub(g, 1, 3), b"KN0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 1, 4), b"KN11") || eq(sub(g, 1, 4), b"KN11") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"Z6") && (eq(sub(g, 1, 4), b"KN02") || eq(sub(g, 1, 4), b"KN03")) {
            f.gvalid = true;
        } else if gt(sub(c, 2, 2), b"J") && lt(sub(c, 2, 2), b"N") {
            if (eq(sub(g, 1, 2), b"RF") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"RE") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"AE16") || eq(sub(g, 1, 4), b"AE15") || eq(sub(g, 1, 4), b"RE31") || eq(sub(g, 1, 4), b"RE90") || eq(sub(g, 1, 4), b"RE92") || eq(sub(g, 1, 4), b"AF10") || eq(sub(g, 1, 4), b"AG09") || eq(sub(g, 1, 4), b"AG08") || eq(sub(g, 1, 4), b"RD29") || eq(sub(g, 1, 4), b"RD39") || eq(sub(g, 1, 4), b"RD47") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"Q") && lt(sub(c, 2, 2), b"V") {
            if (eq(sub(g, 1, 2), b"KG") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")))) || (eq(sub(g, 1, 2), b"KF") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JF") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"85")) || (eq(sub(g, 1, 2), b"JG") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KE83") || eq(sub(g, 1, 4), b"KE93") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"KF") && ((eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 4, 4), b"6") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"57")) { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 2), b"ZP") {
            if (eq(sub(g, 1, 2), b"GG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FG") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"FH90") || eq(sub(g, 1, 4), b"GH00") || eq(sub(g, 1, 4), b"FG95") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"ZA") {
            if (eq(sub(g, 1, 3), b"JN9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 3), b"KN0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 1, 4), b"JM99") || eq(sub(g, 1, 4), b"KM09") { f.gvalid = true; }
        } else if (eq(sub(c, 1, 2), b"ZB") || eq(sub(c, 1, 2), b"ZG")) && eq(sub(g, 1, 4), b"IM76") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"ZC4") && eq(sub(g, 1, 4), b"KM64") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"Z2") {
            if (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"KG") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"Z8") {
            if (eq(sub(g, 1, 2), b"KJ") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 3), b"ZD7") && (eq(sub(g, 1, 4), b"IH74") || eq(sub(g, 1, 4), b"IH73")) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"ZD8") && eq(sub(g, 1, 4), b"II22") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"ZD9") && (eq(sub(g, 1, 4), b"IF32") || eq(sub(g, 1, 4), b"IE49") || eq(sub(g, 1, 4), b"IE59")) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"ZF") && (eq(sub(g, 1, 4), b"EK99") || eq(sub(g, 1, 4), b"FK09")) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"ZK3") && (eq(sub(g, 1, 4), b"AI31") || eq(sub(g, 1, 4), b"AI40")) {
            f.gvalid = true;
        } else if gt(sub(c, 2, 2), b"U") && lt(sub(c, 2, 2), b"[") {
            if (eq(sub(g, 1, 2), b"GG") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"HI") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6") && ne(sub(g, 3, 4), b"25")) || eq(sub(g, 3, 4), b"06") || eq(sub(g, 3, 4), b"07") || eq(sub(g, 3, 4), b"36"))) || (eq(sub(g, 1, 2), b"HH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2")) || (eq(sub(g, 1, 2), b"GH") && ne(sub(g, 3, 4), b"01")) || eq(sub(g, 1, 2), b"GI") || (eq(sub(g, 1, 2), b"GF") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"36"))) || (eq(sub(g, 1, 2), b"GJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"04") || eq(sub(g, 3, 4), b"44"))) || (eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"95"))) || (eq(sub(g, 1, 2), b"FI") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"FH") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"94") || eq(sub(g, 3, 4), b"95"))) || eq(sub(g, 1, 4), b"HG59") || eq(sub(g, 1, 4), b"HJ50") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"GG") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6") && ne(sub(g, 3, 4), b"64") && ne(sub(g, 3, 4), b"65")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"HH") && (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GI") && ((gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GF") && (eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"27")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"GJ") && (eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"52") || eq(sub(g, 3, 4), b"53")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FJ") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FI") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FH") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"68")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 2, 2), b"1") || eq(sub(c, 2, 2), b"7") || eq(sub(c, 2, 2), b"9") {
            f.wrongcall = true;
            return true;
        }
    }
    false
}

#[allow(clippy::all, clippy::pedantic, unused_parens, unreachable_code, unused_variables)]
fn block_4(c: &[u8; 12], g: &[u8; 12], f: &mut Flags) -> bool {
    if eq(sub(c, 1, 1), b"1") {
        if eq(sub(c, 1, 2), b"1A") && eq(sub(g, 1, 4), b"JN61") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"1B") {
            if (eq(sub(g, 1, 3), b"KM6") || eq(sub(g, 1, 3), b"KM7")) && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"6") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"1S") {
            if (eq(sub(g, 1, 2), b"OK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
        } else if (gt(sub(c, 2, 2), b"B") && lt(sub(c, 2, 2), b"S")) || (gt(sub(c, 2, 2), b"S") && lt(sub(c, 2, 2), b"[")) {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"2") {
        if eq(sub(c, 2, 2), b"E") || eq(sub(c, 2, 2), b"A") || eq(sub(c, 2, 2), b"M") || eq(sub(c, 2, 2), b"W") || eq(sub(c, 2, 2), b"I") || eq(sub(c, 2, 2), b"J") || eq(sub(c, 2, 2), b"U") || eq(sub(c, 2, 2), b"D") {
            if (eq(sub(g, 1, 2), b"IO") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 3), b"JO0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 1, 4), b"IP90") || eq(sub(g, 1, 4), b"IN69") || eq(sub(g, 1, 4), b"IN89") || eq(sub(g, 1, 4), b"IO37") { f.gvalid = true; }
            if f.gvalid && eq(sub(g, 1, 2), b"IO") {
                if (eq(sub(g, 1, 3), b"IO5") && ne(sub(g, 4, 4), b"4") && ne(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 3), b"IO6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 3, 4), b"96") || eq(sub(g, 3, 4), b"98") { f.gvalid = false; }
            }
        } else {
            f.wrongcall = true;
            return true;
        }
    } else if eq(sub(c, 1, 1), b"3") {
        if eq(sub(c, 1, 2), b"3Z") {
            if (eq(sub(g, 1, 2), b"JO") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"KO") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"20"))) || eq(sub(g, 1, 4), b"KN09") || eq(sub(g, 1, 4), b"KN19") || eq(sub(g, 1, 4), b"KO20") || eq(sub(g, 1, 4), b"JN99") || eq(sub(g, 1, 4), b"JN89") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"3A") && eq(sub(g, 1, 4), b"JN33") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"3G") {
            if (eq(sub(g, 1, 2), b"FF") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"FG") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8")))) || (eq(sub(g, 1, 2), b"FE") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"FH") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"52"))) || (eq(sub(g, 1, 2), b"FD") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"85"))) || eq(sub(g, 1, 4), b"EF96") || eq(sub(g, 1, 4), b"FF06") || eq(sub(g, 1, 4), b"EG93") || eq(sub(g, 1, 4), b"FG03") || eq(sub(g, 1, 4), b"DG52") || eq(sub(g, 1, 4), b"DG73") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"FF") && eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FE") && (eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"41") || eq(sub(g, 3, 4), b"29")) {
                    f.gvalid = false;
                }
            }
        } else if (eq(sub(c, 1, 3), b"3B6") || eq(sub(c, 1, 3), b"3B7")) && (eq(sub(g, 1, 4), b"LH89") || eq(sub(g, 1, 4), b"LH93")) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"3B8") && (eq(sub(g, 1, 4), b"LG89") || eq(sub(g, 1, 4), b"LH80")) {
            f.gvalid = true;
        } else if eq(sub(c, 1, 3), b"3B9") && eq(sub(g, 1, 4), b"MH10") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"3C") {
            if (eq(sub(g, 1, 3), b"JJ4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 1, 4), b"JJ51") || eq(sub(g, 1, 4), b"JJ52") || eq(sub(g, 1, 4), b"JI28") { f.gvalid = true; }
        } else if eq(sub(c, 1, 3), b"3D2") {
            if (eq(sub(g, 1, 2), b"RH") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 3), b"AH0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 1, 4), b"RG78") || eq(sub(g, 1, 4), b"AG08") || eq(sub(g, 1, 4), b"AG09") { f.gvalid = true; }
        } else if eq(sub(c, 1, 3), b"3DA") {
            if (eq(sub(g, 1, 3), b"KG5") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 1, 4), b"KG63") || eq(sub(g, 1, 4), b"KG64") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"3V") {
            if eq(sub(g, 1, 2), b"JM") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"3W") {
            if (eq(sub(g, 1, 2), b"OL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"OK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"OJ19") || eq(sub(g, 1, 4), b"OL41") || eq(sub(g, 1, 4), b"OL23") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"3X") {
            if (eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"IK") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 1, 4), b"IJ39") { f.gvalid = true; }
        } else if eq(sub(c, 1, 3), b"3Y") && (eq(sub(g, 1, 4), b"JD15") || eq(sub(g, 1, 4), b"EC41")) {
            f.gvalid = true;
        } else if eq(sub(c, 2, 2), b"E") || eq(sub(c, 2, 2), b"F") {
            if (eq(sub(g, 1, 2), b"FJ") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09") || eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"18"))) || (eq(sub(g, 1, 2), b"EJ") && (eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"97") || eq(sub(g, 3, 4), b"98") || eq(sub(g, 3, 4), b"99"))) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"G") && lt(sub(c, 2, 2), b"V") {
            if (len_trim(c) > 5) { return true; }
            if eq(sub(g, 1, 2), b"OL") || (eq(sub(g, 1, 2), b"PL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2")) || eq(sub(g, 1, 2), b"OM") || (eq(sub(g, 1, 2), b"PM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || (eq(sub(g, 1, 2), b"PN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8")) || (eq(sub(g, 1, 2), b"PO") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"OK") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59"))) || eq(sub(g, 1, 2), b"ON") || eq(sub(g, 1, 4), b"OO90") || eq(sub(g, 1, 4), b"OO91") || eq(sub(g, 1, 2), b"NN") || eq(sub(g, 1, 2), b"NM") || (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"MN") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"NL") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7") && ne(sub(g, 3, 4), b"83") && ne(sub(g, 3, 4), b"84") && ne(sub(g, 3, 4), b"85")) || eq(sub(g, 3, 4), b"77") || eq(sub(g, 3, 4), b"87")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"ON") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PN") && ((eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"50") || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"59") || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"MM") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OL") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && eq(sub(g, 4, 4), b"0")) || eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"21")) {
                    f.gvalid = false;
                }
            }
        }
    } else if eq(sub(c, 1, 1), b"4") {
        if eq(sub(c, 1, 2), b"4M") {
            if (eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"60") || eq(sub(g, 3, 4), b"70"))) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 1, 4), b"FK85") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7") && ne(sub(g, 3, 4), b"57")) || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"93")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"FJ") && (eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"72") || eq(sub(g, 3, 4), b"82") || eq(sub(g, 3, 4), b"92") || eq(sub(g, 3, 4), b"91")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"4X") || eq(sub(c, 1, 2), b"4Z") {
            if (eq(sub(g, 1, 3), b"KM7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 1, 4), b"KL79") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"4O") {
            if (eq(sub(g, 1, 3), b"JN9") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"4")) || eq(sub(g, 1, 4), b"KN02") || eq(sub(g, 1, 4), b"KN03") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"O") && lt(sub(c, 2, 2), b"T") {
            if (eq(sub(g, 1, 3), b"MJ9") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"NJ0") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"C") && lt(sub(c, 2, 2), b"J") {
            if (eq(sub(g, 1, 2), b"PK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4")) || (eq(sub(g, 1, 2), b"PJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"04"))) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OK") && eq(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 2), b"PL") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10"))) || eq(sub(g, 1, 4), b"OK70") || eq(sub(g, 1, 4), b"OK71") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"PK") && ((eq(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":"))) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PJ") && (eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"15")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OJ") && (eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"4J") || eq(sub(c, 1, 2), b"4J") {
            if (eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"LN50") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"4J") || eq(sub(c, 1, 2), b"4J") {
            if eq(sub(g, 1, 2), b"LN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4") { f.gvalid = true; }
        } else if eq(sub(c, 2, 2), b"U") && (eq(sub(c, 1, 6), b"4U1ITU") || eq(sub(c, 1, 5), b"4U1UN") || eq(sub(c, 1, 6), b"4U1VIC") || eq(sub(g, 1, 4), b"JN88")) {
            f.gvalid = true;
        } else if gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"D") {
            if (eq(sub(g, 1, 2), b"DL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"EK") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"34"))) || (eq(sub(g, 1, 2), b"DM") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"DK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"DK") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"38"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"DL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"33") || eq(sub(g, 3, 4), b"60")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"62")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EK") && (eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"65") || eq(sub(g, 3, 4), b"66") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DM") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && eq(sub(g, 4, 4), b"2") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DK") && (eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"77") || eq(sub(g, 3, 4), b"86")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"4V") {
            if (eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"38") || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49"))) || eq(sub(g, 1, 4), b"FL30") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"4W") {
            if eq(sub(g, 1, 2), b"PI") && (eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"31")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"4T") {
            if (eq(sub(g, 1, 2), b"FH") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"FI") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 3), b"EI9") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"7")) { f.gvalid = true; }
        }
    } else if eq(sub(c, 1, 1), b"5") {
        if eq(sub(c, 1, 2), b"5P") || eq(sub(c, 1, 2), b"5Q") {
            if eq(sub(g, 1, 2), b"JO") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"76")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5B") {
            if (eq(sub(g, 1, 3), b"KM6") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) || (eq(sub(g, 1, 3), b"KM7") && (eq(sub(g, 4, 4), b"4") || eq(sub(g, 4, 4), b"5"))) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"B") && lt(sub(c, 2, 2), b"H") {
            if (eq(sub(g, 1, 2), b"IM") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 2), b"IL") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"69") || eq(sub(g, 3, 4), b"79"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5A") {
            if (eq(sub(g, 1, 2), b"JM") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"/") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"KM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"JL") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3")) || (eq(sub(g, 1, 2), b"JM") && (eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"53"))) || eq(sub(g, 1, 4), b"JL91") || eq(sub(g, 1, 4), b"KK19") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5H") || eq(sub(c, 1, 2), b"5I") {
            if (eq(sub(g, 1, 2), b"KI") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5K") || eq(sub(c, 1, 2), b"5J") {
            if (eq(sub(g, 1, 2), b"FJ") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"7")) || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"02") || eq(sub(g, 3, 4), b"03"))) || (eq(sub(g, 1, 2), b"FK") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3") && ne(sub(g, 3, 4), b"22")) || (eq(sub(g, 1, 2), b"FI") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"55") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"56")) && ne(sub(g, 3, 4), b"27")) || (eq(sub(g, 1, 2), b"EJ") && (eq(sub(g, 3, 4), b"93") || eq(sub(g, 3, 4), b"94"))) || eq(sub(g, 1, 4), b"EK92") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5L") || eq(sub(c, 1, 2), b"5M") {
            if eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"67") && ne(sub(g, 3, 4), b"67") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5N") || eq(sub(c, 1, 2), b"5O") {
            if (eq(sub(g, 1, 2), b"JJ") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5R") || eq(sub(c, 1, 2), b"5S") {
            if (eq(sub(g, 1, 2), b"LH") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 2), b"LG") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"LH") && (eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"47") || eq(sub(g, 3, 4), b"48")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5T") {
            if (eq(sub(g, 1, 2), b"IK") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"44"))) || (eq(sub(g, 1, 2), b"IL") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"66") || eq(sub(g, 3, 4), b"57") || eq(sub(g, 3, 4), b"75"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"IK") && eq(sub(g, 3, 4), b"15") || eq(sub(g, 3, 4), b"25") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"IL") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6")) || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"35")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"5U") {
            if (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JL") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || eq(sub(g, 1, 4), b"JK11") || eq(sub(g, 1, 2), b"JL") && (eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"53") || eq(sub(g, 3, 4), b"63")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5V") {
            if (eq(sub(g, 1, 3), b"JJ0") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JK") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01"))) || (eq(sub(g, 1, 2), b"IK") && (eq(sub(g, 3, 4), b"90") || eq(sub(g, 3, 4), b"91"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5W") {
            if eq(sub(g, 1, 2), b"AH") && (eq(sub(g, 3, 4), b"36") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"45")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5X") {
            if (eq(sub(g, 1, 2), b"KJ") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (eq(sub(g, 1, 2), b"KI") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && eq(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"KJ") && (eq(sub(g, 3, 4), b"40") || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"74"))) || (eq(sub(g, 1, 2), b"KI") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"58"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"5Z") || eq(sub(c, 1, 2), b"5Y") {
            if (eq(sub(g, 1, 2), b"KI") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KJ") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"LI") && eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LJ") && eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 1, 4), b"KJ60") || eq(sub(g, 1, 4), b"KI69") { f.gvalid = true; }
        }
    } else if eq(sub(c, 1, 1), b"6") {
        if eq(sub(c, 1, 2), b"6C") {
            if (eq(sub(g, 1, 2), b"KM") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"C") && lt(sub(c, 2, 2), b"K") {
            if (eq(sub(g, 1, 2), b"DL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"09"))) || (eq(sub(g, 1, 2), b"EL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"EK") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"34"))) || (eq(sub(g, 1, 2), b"DM") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"DK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"DK") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"28") || eq(sub(g, 3, 4), b"38"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"DL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"23") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"33") || eq(sub(g, 3, 4), b"60")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EL") && ((gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"20") || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"32") || eq(sub(g, 3, 4), b"62")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"EK") && (eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"65") || eq(sub(g, 3, 4), b"66") || eq(sub(g, 3, 4), b"67") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DM") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && eq(sub(g, 4, 4), b"2") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"DK") && (eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"77") || eq(sub(g, 3, 4), b"86")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 1), b"6") && gt(sub(c, 2, 2), b"J") && lt(sub(c, 2, 2), b"O") {
            if (eq(sub(g, 1, 3), b"PM3") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 3), b"PM4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 1, 4), b"PM24") || eq(sub(g, 1, 4), b"PM57") || eq(sub(g, 1, 4), b"GC07") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"6V") || eq(sub(c, 1, 2), b"6W") {
            if eq(sub(g, 1, 2), b"IK") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"42") || eq(sub(g, 3, 4), b"43")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"6X") {
            if (eq(sub(g, 1, 2), b"LH") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || (eq(sub(g, 1, 2), b"LG") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 2), b"LH") && (eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"47") || eq(sub(g, 3, 4), b"48")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"6Y") {
            if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"17") || eq(sub(g, 3, 4), b"18") || eq(sub(g, 3, 4), b"08") || eq(sub(g, 3, 4), b"06")) { f.gvalid = true; }
        } else if eq(sub(c, 2, 2), b"T") || eq(sub(c, 2, 2), b"U") {
            if (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"KL") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KJ79") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"O") && lt(sub(c, 2, 2), b"T") {
            if (eq(sub(g, 1, 2), b"MM") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"31") || eq(sub(g, 3, 4), b"84") || eq(sub(g, 3, 4), b"85"))) || (eq(sub(g, 1, 2), b"ML") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"33"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"MM") && eq(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"7") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"ML") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 3, 4), b"07")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"6Z") {
            if eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"9") && ne(sub(g, 3, 4), b"67") && ne(sub(g, 3, 4), b"67") { f.gvalid = true; }
        }
    } else if eq(sub(c, 1, 1), b"7") {
        if eq(sub(c, 1, 1), b"7") && gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"J") {
            if (eq(sub(g, 1, 2), b"NJ") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"NI") && gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OI") && !(gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) && !(gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && eq(sub(g, 4, 4), b"0"))) || (eq(sub(g, 1, 2), b"PI") && !(gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && eq(sub(g, 4, 4), b"0"))) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"PJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"QI") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"PH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"PJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NJ") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"7") && ((gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"3"))) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OJ") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"04")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OI") && ((eq(sub(g, 4, 4), b"1") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PI") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"83") || eq(sub(g, 3, 4), b"25") || eq(sub(g, 3, 4), b"35") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"29")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 1), b"7") && gt(sub(c, 2, 2), b"I") && lt(sub(c, 2, 2), b"O") {
            if (eq(sub(g, 1, 2), b"PM") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 3), b"QM0") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"QN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 3), b"PN9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"PL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"QM19") || eq(sub(g, 1, 4), b"KC90") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"PM") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3") && ne(sub(g, 3, 4), b"62")) || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"55") || eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PL") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"38") || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"57") || eq(sub(g, 3, 4), b"59")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"QN") && ((eq(sub(g, 3, 3), b"2") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"5"))) || eq(sub(g, 3, 4), b"10")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"7O") {
            if eq(sub(g, 1, 2), b"LK") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 3, 4), b"05") || eq(sub(g, 3, 4), b"06") || eq(sub(g, 3, 4), b"12") || eq(sub(g, 3, 4), b"22")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"7P") {
            if (eq(sub(g, 1, 2), b"KG") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"KF") && (eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"49"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"7Q") {
            if (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"KI60") || (eq(sub(g, 1, 3), b"KH7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"7S") {
            if (eq(sub(g, 1, 2), b"JO") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JP") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"KP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"25"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"85") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"JP") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"99")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KP") && (eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"14")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"7R") || gt(sub(c, 2, 2), b"S") && lt(sub(c, 2, 2), b"Z") {
            if (eq(sub(g, 1, 2), b"JM") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"37")) && ne(sub(g, 3, 4), b"43")) || (eq(sub(g, 1, 2), b"IM") && ((gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || eq(sub(g, 3, 4), b"70"))) || (eq(sub(g, 1, 2), b"JL") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || eq(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"1") && eq(sub(g, 4, 4), b"5"))) || (eq(sub(g, 1, 2), b"IL") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"92"))) || (eq(sub(g, 1, 2), b"JK") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"4") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"IL") && ((eq(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"2") && eq(sub(g, 4, 4), b"6")) || eq(sub(g, 3, 4), b"63") || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"73")) { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 2), b"7Z") {
            if (eq(sub(g, 1, 2), b"LL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8")) || (eq(sub(g, 1, 2), b"KL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"LK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10"))) || (eq(sub(g, 1, 2), b"KM") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KM92") { f.gvalid = true; }
        }
    } else if eq(sub(c, 1, 1), b"8") {
        if eq(sub(c, 1, 1), b"8") && gt(sub(c, 2, 2), b"@") && lt(sub(c, 2, 2), b"J") {
            if (eq(sub(g, 1, 2), b"NJ") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"NI") && gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"OI") && !(gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) && !(gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"8") && eq(sub(g, 4, 4), b"0"))) || (eq(sub(g, 1, 2), b"PI") && !(gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":") && eq(sub(g, 4, 4), b"0"))) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"PJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"QI") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || (eq(sub(g, 1, 2), b"PH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && eq(sub(g, 4, 4), b"9")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"PJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NJ") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 3, 3), b"7") && ((gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"3"))) || (eq(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"43") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OJ") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 3, 3), b"1") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"24") || eq(sub(g, 3, 4), b"04")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"OI") && ((eq(sub(g, 4, 4), b"1") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 4, 4), b"5") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"6")) || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"39")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PI") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"62") || eq(sub(g, 3, 4), b"71") || eq(sub(g, 3, 4), b"83") || eq(sub(g, 3, 4), b"25") || eq(sub(g, 3, 4), b"35") || eq(sub(g, 3, 4), b"34") || eq(sub(g, 3, 4), b"44") || eq(sub(g, 3, 4), b"29")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 1), b"8") && gt(sub(c, 2, 2), b"I") && lt(sub(c, 2, 2), b"O") {
            if (eq(sub(g, 1, 2), b"PM") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 3), b"QM0") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"QN") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"6")) || (eq(sub(g, 1, 3), b"PN9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3")) || (eq(sub(g, 1, 2), b"PL") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"QM19") || eq(sub(g, 1, 4), b"KC90") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"PM") && ((gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || (gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"3") && ne(sub(g, 3, 4), b"62")) || eq(sub(g, 3, 4), b"45") || eq(sub(g, 3, 4), b"46") || eq(sub(g, 3, 4), b"55") || eq(sub(g, 3, 4), b"56") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"89")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"PL") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"38") || eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"57") || eq(sub(g, 3, 4), b"59")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"QN") && ((eq(sub(g, 3, 3), b"2") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"1") || eq(sub(g, 4, 4), b"5"))) || eq(sub(g, 3, 4), b"10")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"8O") {
            if (eq(sub(g, 1, 2), b"KG") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"KH") && (eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"30"))) || (eq(sub(g, 1, 2), b"KG") && (eq(sub(g, 3, 4), b"48") || eq(sub(g, 3, 4), b"47"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"8P") && eq(sub(g, 1, 4), b"GK03") {
            f.gvalid = true;
        } else if eq(sub(c, 1, 2), b"8Q") {
            if (eq(sub(g, 1, 3), b"MJ6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"8")) || eq(sub(g, 1, 4), b"MI69") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"8R") {
            if (eq(sub(g, 1, 2), b"GJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 3), b"FJ9") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"8")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"8S") {
            if (eq(sub(g, 1, 2), b"JO") && gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"JP") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 1, 2), b"KP") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"25"))) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"JO") && (eq(sub(g, 3, 4), b"85") || eq(sub(g, 3, 4), b"95")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"JP") && ((eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"9")) || (eq(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9")) || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"99")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"KP") && (eq(sub(g, 3, 4), b"13") || eq(sub(g, 3, 4), b"14")) {
                    f.gvalid = false;
                }
            }
        } else if gt(sub(c, 2, 2), b"S") && lt(sub(c, 2, 2), b"Z") {
            if (eq(sub(g, 1, 2), b"MJ") && (eq(sub(g, 3, 4), b"99") || eq(sub(g, 3, 4), b"89") || eq(sub(g, 3, 4), b"88") || eq(sub(g, 3, 4), b"68"))) || (eq(sub(g, 1, 2), b"MK") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":")) || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"52"))) || (eq(sub(g, 1, 2), b"NK") && ((eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6")) || (gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"3") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"39"))) || (eq(sub(g, 1, 2), b"ML") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || (eq(sub(g, 1, 2), b"MM") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 1, 4), b"NM00") { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"MK") && eq(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"5") {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NK") && (eq(sub(g, 3, 4), b"26") || eq(sub(g, 3, 4), b"27")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"ML") && ((gt(sub(g, 3, 3), b"4") && (eq(sub(g, 4, 4), b"0") || eq(sub(g, 4, 4), b"5") || eq(sub(g, 4, 4), b"8") || eq(sub(g, 4, 4), b"9"))) || eq(sub(g, 3, 4), b"58") || eq(sub(g, 3, 4), b"59")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"NL") && ((eq(sub(g, 4, 4), b"9") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 4, 4), b"8") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"40")) {
                    f.gvalid = false;
                } else if eq(sub(g, 1, 2), b"MM") && (eq(sub(g, 3, 4), b"61") || eq(sub(g, 3, 4), b"62")) {
                    f.gvalid = false;
                }
            }
        } else if eq(sub(c, 1, 2), b"8Z") {
            if (eq(sub(g, 1, 2), b"LL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8")) || (eq(sub(g, 1, 2), b"KL") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"LK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"01") || eq(sub(g, 3, 4), b"10"))) || (eq(sub(g, 1, 2), b"KM") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KM92") { f.gvalid = true; }
        }
    } else if eq(sub(c, 1, 1), b"9") {
        if eq(sub(c, 1, 2), b"9A") {
            if eq(sub(g, 1, 2), b"JN") && ((gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b"6")) || eq(sub(g, 3, 4), b"64") || eq(sub(g, 3, 4), b"65") || eq(sub(g, 3, 4), b"76") || eq(sub(g, 3, 4), b"86")) && ne(sub(g, 3, 4), b"93") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9H") {
            if eq(sub(g, 1, 2), b"JM") && (eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"76")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9M") || eq(sub(c, 1, 2), b"9W") {
            if eq(sub(g, 1, 4), b"NJ96") || (eq(sub(g, 1, 2), b"OJ") && (((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2")) || (gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":"))) && gt(sub(g, 4, 4), b"0") && lt(sub(g, 4, 4), b"7")) || eq(sub(g, 3, 4), b"21") || eq(sub(g, 3, 4), b"22") || eq(sub(g, 3, 4), b"41") || eq(sub(g, 3, 4), b"87")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9K") {
            if (eq(sub(g, 1, 2), b"LL") && (eq(sub(g, 3, 4), b"39") || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"38") || eq(sub(g, 3, 4), b"48"))) || (eq(sub(g, 1, 2), b"LM") && (eq(sub(g, 3, 4), b"30") || eq(sub(g, 3, 4), b"40"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9V") {
            if eq(sub(g, 1, 2), b"OJ") && (eq(sub(g, 3, 4), b"11") || eq(sub(g, 3, 4), b"21")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9Y") || eq(sub(c, 1, 2), b"9Z") {
            if eq(sub(g, 1, 2), b"FK") && (eq(sub(g, 3, 4), b"90") || eq(sub(g, 3, 4), b"91")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9N") {
            if (eq(sub(g, 1, 2), b"NL") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"NM") && (eq(sub(g, 3, 4), b"00") || eq(sub(g, 3, 4), b"10"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9G") {
            if (eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 3), b"JJ0") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"IK") && gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && eq(sub(g, 4, 4), b"0")) || (eq(sub(g, 1, 4), b"JK00") || eq(sub(g, 1, 4), b"JJ84") || eq(sub(g, 1, 4), b"JJ94")) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9J") || eq(sub(c, 1, 2), b"9I") {
            if (eq(sub(g, 1, 2), b"KH") && gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KI") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"7") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || eq(sub(g, 1, 4), b"KH31") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9L") {
            if (eq(sub(g, 1, 2), b"IJ") && gt(sub(g, 3, 3), b"2") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 1, 4), b"IJ46") { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"N") && lt(sub(c, 2, 2), b"U") {
            if (eq(sub(g, 1, 2), b"JI") && ((gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"8") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b"6")) || gt(sub(g, 3, 3), b"7") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"1") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"81")) || (eq(sub(g, 1, 2), b"KI") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"5")) || eq(sub(g, 3, 4), b"51") || eq(sub(g, 3, 4), b"52"))) || (eq(sub(g, 1, 2), b"KJ") && ((gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"25") || eq(sub(g, 3, 4), b"35"))) || (eq(sub(g, 1, 2), b"JJ") && ((gt(sub(g, 3, 3), b"8") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || eq(sub(g, 3, 4), b"80") || eq(sub(g, 3, 4), b"81") || eq(sub(g, 3, 4), b"95"))) || (eq(sub(g, 1, 2), b"KH") && ((gt(sub(g, 3, 3), b"0") && lt(sub(g, 3, 3), b"5") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"37") || eq(sub(g, 3, 4), b"47") || eq(sub(g, 3, 4), b"46"))) { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9U") {
            if eq(sub(g, 1, 2), b"KI") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b"8") { f.gvalid = true; }
        } else if eq(sub(c, 1, 2), b"9X") {
            if eq(sub(g, 1, 2), b"KI") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"6") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b"9") { f.gvalid = true; }
        } else if eq(sub(c, 1, 3), b"9M0") {
            if (eq(sub(g, 1, 2), b"OK") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"2")) || (eq(sub(g, 1, 2), b"OJ") && gt(sub(g, 3, 3), b"3") && lt(sub(g, 3, 3), b"9") && gt(sub(g, 4, 4), b"6") && lt(sub(g, 4, 4), b":")) { f.gvalid = true; }
        } else if gt(sub(c, 2, 2), b"A") && lt(sub(c, 2, 2), b"E") {
            if (eq(sub(g, 1, 2), b"LM") && gt(sub(g, 3, 3), b"1") && lt(sub(g, 3, 3), b":")) || (eq(sub(g, 1, 2), b"LL") && ((gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"5") && lt(sub(g, 4, 4), b":")) || eq(sub(g, 3, 4), b"49") || eq(sub(g, 3, 4), b"75") || eq(sub(g, 3, 4), b"85") || eq(sub(g, 3, 4), b"95"))) || (eq(sub(g, 1, 2), b"ML") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"4") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"15") && ne(sub(g, 3, 4), b"19")) || (eq(sub(g, 1, 2), b"MM") && eq(sub(g, 3, 3), b"0") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"7")) { f.gvalid = true; }
            if f.gvalid {
                if eq(sub(g, 1, 2), b"LM") && ((eq(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"4")) || (gt(sub(g, 3, 3), b"4") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"7") && lt(sub(g, 4, 4), b":") && ne(sub(g, 3, 4), b"78") && ne(sub(g, 3, 4), b"88"))) { f.gvalid = false; }
            }
        } else if eq(sub(c, 1, 2), b"9E") || eq(sub(c, 1, 2), b"9F") {
            if (eq(sub(g, 1, 2), b"LJ") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"4") && gt(sub(g, 4, 4), b"3") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"LK") && gt(sub(g, 3, 3), b"/") && lt(sub(g, 3, 3), b"2") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) || (eq(sub(g, 1, 2), b"KJ") && gt(sub(g, 3, 3), b"5") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"2") && lt(sub(g, 4, 4), b":")) || (eq(sub(g, 1, 2), b"KK") && gt(sub(g, 3, 3), b"6") && lt(sub(g, 3, 3), b":") && gt(sub(g, 4, 4), b"/") && lt(sub(g, 4, 4), b"5")) { f.gvalid = true; }
        }
    }
    false
}
