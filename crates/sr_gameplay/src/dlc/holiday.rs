//! Port of `HolidayDirector.cs` and `HolidayModel.cs` (Party Gordo weekends & Wiggly Wonderland Echo Note Gordos).

use serde::{Deserialize, Serialize};
use sr_data::GameModeSettings;
use sr_ids::{IdentifiableId, PediaId};
use sr_math::Randoms;
use sr_model::{EchoNoteGordoModel, GameModel, GordoModel};

use crate::story::PediaDirector;

pub const PARTY_CRATE_CHANCE: f32 = 0.5;
pub const RARE_ORNAMENT_CHANCE: f32 = 0.05;
pub const PARTY_GORDO_TARGET_FOOD: i32 = 10;

pub const RARE_ORNAMENTS: &[IdentifiableId] = &[
    IdentifiableId::GoldOrnament,
    IdentifiableId::LuckyOrnament,
    IdentifiableId::PinkPartyOrnament,
    IdentifiableId::RainbowOrnament,
    IdentifiableId::TarrOrnament,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayDate {
    pub year: i32,
    /// Month `1..=12`
    pub month: u8,
    /// Day of month `1..=31`
    pub day: u8,
    /// ISO day of week `1..=7` (`1 = Monday`, `5 = Friday`, `6 = Saturday`, `7 = Sunday`)
    pub iso_weekday: u8,
    /// ISO-like week of year `1..=53`
    pub week_number: u8,
}

impl HolidayDate {
    pub const fn new(year: i32, month: u8, day: u8, iso_weekday: u8, week_number: u8) -> Self {
        Self {
            year,
            month,
            day,
            iso_weekday,
            week_number,
        }
    }

    /// Computes a Gregorian `HolidayDate` from UTC Unix seconds (`1970-01-01` = Thursday, `iso_weekday = 4`).
    pub fn from_unix_timestamp(unix_secs: i64) -> Self {
        let days = unix_secs.div_euclid(86_400);
        // Howard Hinnant civil-from-days algorithm
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = (z - era * 146_097) as u32;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let mut year = yoe as i32 + (era as i32) * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
        if month <= 2 {
            year += 1;
        }
        // 1970-01-01 was Thursday (4)
        let iso_weekday = ((days + 3).rem_euclid(7) + 1) as u8;
        // Day of year (1..=366) -> simple week_number matching C# CalendarWeekRule
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let cum_days: [u16; 12] = if is_leap {
            [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335]
        } else {
            [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]
        };
        let jan1_doy = cum_days[(month - 1) as usize] + u16::from(day);
        let week_number = (((jan1_doy - 1) / 7) + 1).min(53) as u8;
        Self {
            year,
            month,
            day,
            iso_weekday,
            week_number,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventGordoDef {
    pub object_id: &'static str,
    pub week_number: u8,
    pub start_day: u8,
    pub final_day: u8,
    pub commons: [IdentifiableId; 2],
}

impl EventGordoDef {
    pub fn event_id(&self) -> String {
        format!(
            "{}:W{}:{}-{}",
            self.object_id, self.week_number, self.start_day, self.final_day
        )
    }

    pub fn is_live(&self, date: HolidayDate) -> bool {
        date.week_number == self.week_number
            && date.iso_weekday >= self.start_day
            && date.iso_weekday <= self.final_day
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventEchoNoteGordoDef {
    pub object_id: &'static str,
    pub month: u8,
    pub day: u8,
    pub echo_note: IdentifiableId,
}

impl EventEchoNoteGordoDef {
    pub fn event_id(&self) -> String {
        format!("{}:{:02}{:02}", self.object_id, self.month, self.day)
    }

    pub fn is_live(&self, date: HolidayDate) -> bool {
        date.month == self.month && date.day == self.day
    }
}

/// Subset of the canonical weekly Party Gordo schedule (representative weeks + Twinkle Dec 18..30).
pub const PARTY_GORDO_SCHEDULE: &[EventGordoDef] = &[
    EventGordoDef {
        object_id: "gordoPinkParty19",
        week_number: 1,
        start_day: 5,
        final_day: 7,
        commons: [IdentifiableId::GoldOrnament, IdentifiableId::LuckyOrnament],
    },
    EventGordoDef {
        object_id: "gordoPinkParty24",
        week_number: 2,
        start_day: 5,
        final_day: 7,
        commons: [
            IdentifiableId::SaberOrnament,
            IdentifiableId::StripesPurpleOrnament,
        ],
    },
    EventGordoDef {
        object_id: "gordoPinkParty16",
        week_number: 3,
        start_day: 5,
        final_day: 7,
        commons: [IdentifiableId::BoomOrnament, IdentifiableId::TabbyOrnament],
    },
    EventGordoDef {
        object_id: "gordoPinkParty27",
        week_number: 6,
        start_day: 5,
        final_day: 7,
        commons: [
            IdentifiableId::GlitchOrnament,
            IdentifiableId::PhosphorOrnament,
        ],
    },
    EventGordoDef {
        object_id: "gordoPinkParty15",
        week_number: 50,
        start_day: 5,
        final_day: 7,
        commons: [
            IdentifiableId::StripesRedOrnament,
            IdentifiableId::StripesGreenOrnament,
        ],
    },
    EventGordoDef {
        object_id: "gordoPinkParty13",
        week_number: 51,
        start_day: 5,
        final_day: 7,
        commons: [
            IdentifiableId::SnowflakeOrnament,
            IdentifiableId::StarOrnament,
        ],
    },
    EventGordoDef {
        object_id: "gordoPinkParty29",
        week_number: 52,
        start_day: 5,
        final_day: 7,
        commons: [
            IdentifiableId::QuicksilverOrnament,
            IdentifiableId::CloudOrnament,
        ],
    },
    EventGordoDef {
        object_id: "gordoPinkParty2",
        week_number: 53,
        start_day: 5,
        final_day: 7,
        commons: [
            IdentifiableId::TwinkleOrnament,
            IdentifiableId::TwinkleOrnament,
        ],
    },
];

pub const ECHO_NOTE_GORDO_SCHEDULE: &[EventEchoNoteGordoDef] = &[
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote9",
        month: 12,
        day: 18,
        echo_note: IdentifiableId::EchoNote09,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote1",
        month: 12,
        day: 19,
        echo_note: IdentifiableId::EchoNote01,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote6",
        month: 12,
        day: 20,
        echo_note: IdentifiableId::EchoNote06,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote8",
        month: 12,
        day: 21,
        echo_note: IdentifiableId::EchoNote08,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote10",
        month: 12,
        day: 22,
        echo_note: IdentifiableId::EchoNote10,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote4",
        month: 12,
        day: 23,
        echo_note: IdentifiableId::EchoNote04,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote13",
        month: 12,
        day: 24,
        echo_note: IdentifiableId::EchoNote13,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote11",
        month: 12,
        day: 25,
        echo_note: IdentifiableId::EchoNote11,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote7",
        month: 12,
        day: 26,
        echo_note: IdentifiableId::EchoNote07,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote2",
        month: 12,
        day: 27,
        echo_note: IdentifiableId::EchoNote02,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote12",
        month: 12,
        day: 28,
        echo_note: IdentifiableId::EchoNote12,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote5",
        month: 12,
        day: 29,
        echo_note: IdentifiableId::EchoNote05,
    },
    EventEchoNoteGordoDef {
        object_id: "gordoEchoNote3",
        month: 12,
        day: 30,
        echo_note: IdentifiableId::EchoNote03,
    },
];

/// Port of `HolidayDirector`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HolidayDirector {
    pub active_date: Option<HolidayDate>,
}

impl HolidayDirector {
    /// Returns the currently live `EventGordoDef`, if `date` falls on an active weekend slot.
    pub fn find_live_party_gordo(date: HolidayDate) -> Option<&'static EventGordoDef> {
        PARTY_GORDO_SCHEDULE.iter().find(|e| e.is_live(date))
    }

    /// Returns the currently live `EventEchoNoteGordoDef`, if `date` is Dec 18..=30.
    pub fn find_live_echo_gordo(date: HolidayDate) -> Option<&'static EventEchoNoteGordoDef> {
        ECHO_NOTE_GORDO_SCHEDULE.iter().find(|e| e.is_live(date))
    }

    /// Synchronizes `HolidayModel`, `GordoModel`, and `EchoNoteGordoModel` for `date`.
    pub fn sync_holidays(&mut self, model: &mut GameModel, date: HolidayDate) {
        self.active_date = Some(date);
        model.holiday.event_gordos.clear();
        model.holiday.event_echo_note_gordos.clear();

        if let Some(pg) = Self::find_live_party_gordo(date) {
            model.holiday.event_gordos.insert(pg.event_id());
            model
                .gordos
                .entry(pg.object_id.to_string())
                .or_insert_with(|| GordoModel {
                    id: pg.object_id.to_string(),
                    gordo_eaten_count: 0,
                    target_count: PARTY_GORDO_TARGET_FOOD,
                    fashions: Vec::new(),
                });
        }

        if let Some(eg) = Self::find_live_echo_gordo(date) {
            model.holiday.event_echo_note_gordos.insert(eg.event_id());
            model
                .echo_note_gordos
                .entry(eg.object_id.to_string())
                .or_insert_with(|| EchoNoteGordoModel {
                    id: eg.object_id.to_string(),
                    has_been_triggered: false,
                });
        }
    }

    /// Rolls ornament & party crate drops when a `PartyGordo` or `CRATE_PARTY_01` bursts.
    pub fn roll_party_ornament(def: &EventGordoDef, rng: &mut Randoms) -> IdentifiableId {
        if rng.get_probability(RARE_ORNAMENT_CHANCE) {
            let idx = rng.get_in_range_i32(0, RARE_ORNAMENTS.len() as i32) as usize;
            RARE_ORNAMENTS[idx.min(RARE_ORNAMENTS.len() - 1)]
        } else {
            let idx = rng.get_in_range_i32(0, def.commons.len() as i32) as usize;
            def.commons[idx.min(def.commons.len() - 1)]
        }
    }

    /// Triggers an `EchoNoteGordo` (Twinkle Slime), returning the dropped `EchoNote*` and unlocking Pedia entries.
    pub fn trigger_echo_note_gordo(
        model: &mut GameModel,
        mode: &GameModeSettings,
        pedia: &mut PediaDirector,
        object_id: &str,
    ) -> Option<IdentifiableId> {
        let def = ECHO_NOTE_GORDO_SCHEDULE
            .iter()
            .find(|e| e.object_id == object_id)?;
        let gordo = model.echo_note_gordos.get_mut(object_id)?;
        if gordo.has_been_triggered {
            return None;
        }
        gordo.has_been_triggered = true;
        pedia.maybe_show_popup(model, mode, PediaId::EchoNoteGordoSlime);
        pedia.maybe_show_popup(model, mode, PediaId::EchoNotes);
        Some(def.echo_note)
    }
}
