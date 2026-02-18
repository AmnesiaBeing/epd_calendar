#![no_std]

extern crate alloc;

use alloc::string::String;
use core::sync::atomic::{AtomicU32, Ordering};

mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated_hitokoto_data.rs"));
}

static QUOTE_INDEX: AtomicU32 = AtomicU32::new(0);

pub fn get_random_quote() -> Option<Quote<'static>> {
    let hitokotos = generated::HITOKOTOS;
    if hitokotos.is_empty() {
        return None;
    }

    let index = (QUOTE_INDEX.fetch_add(1, Ordering::Relaxed) as usize) % hitokotos.len();
    let hitokoto = &hitokotos[index];

    let text = hitokoto.hitokoto;
    let from = generated::FROM_STRINGS
        .get(hitokoto.from as usize)
        .unwrap_or(&"");
    let from_who = generated::FROM_WHO_STRINGS
        .get(hitokoto.from_who as usize)
        .unwrap_or(&"");
    let from_who = if from_who.is_empty() || *from_who == "佚 名" {
        ""
    } else {
        from_who
    };

    Some(Quote {
        text,
        from,
        from_who,
    })
}

pub fn get_daily_quote(day_of_year: u16) -> Option<Quote<'static>> {
    let hitokotos = generated::HITOKOTOS;
    if hitokotos.is_empty() {
        return None;
    }

    let index = (day_of_year as usize) % hitokotos.len();
    let hitokoto = &hitokotos[index];

    let text = hitokoto.hitokoto;
    let from = generated::FROM_STRINGS
        .get(hitokoto.from as usize)
        .unwrap_or(&"");
    let from_who = generated::FROM_WHO_STRINGS
        .get(hitokoto.from_who as usize)
        .unwrap_or(&"");
    let from_who = if from_who.is_empty() || *from_who == "佚 名" {
        ""
    } else {
        from_who
    };

    Some(Quote {
        text,
        from,
        from_who,
    })
}

pub fn get_quote_count() -> usize {
    generated::HITOKOTOS.len()
}

#[derive(Clone, Copy)]
pub struct Quote<'a> {
    pub text: &'a str,
    pub from: &'a str,
    pub from_who: &'a str,
}

impl Quote<'_> {
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        result.push_str(self.text);
        if !self.from.is_empty() {
            result.push_str(" —— ");
            result.push_str(self.from);
            if !self.from_who.is_empty() {
                result.push_str(" ");
                result.push_str(self.from_who);
            }
        }
        result
    }
}
