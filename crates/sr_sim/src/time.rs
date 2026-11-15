//! Port of `TimeDirector.cs` and `TimeUtil.cs`.

use sr_model::WorldModel;

pub const SECS_PER_MIN: f64 = 60.0;
pub const MINS_PER_HOUR: f64 = 60.0;
pub const HOURS_PER_DAY: f64 = 24.0;
pub const MINS_PER_DAY: f64 = 1440.0;
pub const SECS_PER_HOUR: f64 = 3600.0;
pub const SECS_PER_DAY: f64 = 86400.0;

pub const DEFAULT_SECS_PER_GAME_DAY: f64 = 1440.0;
pub const DEFAULT_FF_SECS_PER_GAME_DAY: f64 = 5.0;
pub const START_HOUR: f64 = 9.0;
pub const START_WORLD_TIME: f64 = START_HOUR * SECS_PER_HOUR; // 32400.0

/// Coarse icon bucket returned by `TimeDirector::curr_time_icon`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeOfDayIcon {
    Night,
    Dawn,
    Day,
    Dusk,
}

/// Port of `TimeDirector` driving `WorldModel::world_time`, fast-forward sleep, and pause stack.
#[derive(Debug, Clone)]
pub struct TimeDirector {
    pub secs_per_game_day: f64,
    pub ff_secs_per_game_day: f64,
    pub time_factor: f64,
    pub ff_time_factor: f64,
    pub pauser_count: i32,
    pub special_pauser_count: i32,
    pub time_scale: f32,
    saved_time_scale: f32,
}

impl Default for TimeDirector {
    fn default() -> Self {
        Self::new(DEFAULT_SECS_PER_GAME_DAY, DEFAULT_FF_SECS_PER_GAME_DAY)
    }
}

impl TimeDirector {
    pub fn new(secs_per_game_day: f64, ff_secs_per_game_day: f64) -> Self {
        Self {
            secs_per_game_day,
            ff_secs_per_game_day,
            time_factor: SECS_PER_DAY / secs_per_game_day,
            ff_time_factor: SECS_PER_DAY / ff_secs_per_game_day,
            pauser_count: 0,
            special_pauser_count: 0,
            time_scale: 1.0,
            saved_time_scale: 1.0,
        }
    }

    /// Port of `TimeDirector.InitModel` + `ResetToStartTime`.
    pub fn init_world_model(&self, world: &mut WorldModel) {
        world.fast_forward_until = None;
        world.pause_world_time = false;
        world.world_time = START_WORLD_TIME;
        world.last_world_time = Some(world.world_time);
    }

    pub fn is_paused(&self) -> bool {
        self.time_scale == 0.0 || self.pauser_count > 0 || self.special_pauser_count > 0
    }

    pub fn pause(&mut self, pause_special_scenes: bool) {
        let was_paused = self.pauser_count > 0 || self.special_pauser_count > 0;
        self.pauser_count += 1;
        if pause_special_scenes {
            self.special_pauser_count += 1;
        }
        let now_paused = self.pauser_count > 0 || self.special_pauser_count > 0;
        if !was_paused && now_paused {
            self.saved_time_scale = self.time_scale;
            self.time_scale = 0.0;
        }
    }

    pub fn unpause(&mut self, pause_special_scenes: bool) {
        let was_paused = self.pauser_count > 0 || self.special_pauser_count > 0;
        self.pauser_count -= 1;
        if pause_special_scenes {
            self.special_pauser_count -= 1;
        }
        if self.pauser_count < 0 {
            self.pauser_count = 0;
            return;
        }
        let now_paused = self.pauser_count > 0 || self.special_pauser_count > 0;
        if was_paused && !now_paused {
            self.time_scale = self.saved_time_scale;
        }
    }

    pub fn exactly_one_pauser(&self) -> bool {
        self.pauser_count == 1
    }

    pub fn has_pauser(&self) -> bool {
        self.pauser_count > 0
    }

    /// Port of `TimeDirector.Update` (returns `Some(false)` when fast-forward finished this tick).
    pub fn update(&mut self, world: &mut WorldModel, dt: f32) -> Option<bool> {
        if self.time_scale == 0.0 || world.pause_world_time {
            return None;
        }
        let scaled_dt = (dt as f64) * (self.time_scale as f64);
        if let Some(target) = world.fast_forward_until {
            world.world_time += scaled_dt * self.ff_time_factor;
            if world.world_time >= target {
                world.world_time = target;
                world.fast_forward_until = None;
                return Some(false);
            }
        } else {
            world.world_time += scaled_dt * self.time_factor;
        }
        None
    }

    /// Port of `TimeDirector.LateUpdate` (commits `last_world_time`).
    pub fn late_update(&self, world: &mut WorldModel) {
        world.last_world_time = Some(world.world_time);
    }

    pub fn delta_world_time(world: &WorldModel) -> f64 {
        match world.last_world_time {
            Some(prev) => world.world_time - prev,
            None => 0.0,
        }
    }

    pub fn fast_forward_to(world: &mut WorldModel, fast_forward_until: f64) {
        let rounded = (fast_forward_until + 0.5).floor();
        world.fast_forward_until = Some(rounded);
    }

    pub fn is_fast_forwarding(world: &WorldModel) -> bool {
        world.fast_forward_until.is_some()
    }

    #[inline]
    pub fn has_reached(world: &WorldModel, target_world_time: f64) -> bool {
        world.world_time >= target_world_time
    }

    #[inline]
    pub fn hours_from_now(world: &WorldModel, hours: f32) -> f64 {
        world.world_time + (hours as f64) * SECS_PER_HOUR
    }

    #[inline]
    pub fn hours_from_time(hours: f32, time: f64) -> f64 {
        time + (hours as f64) * SECS_PER_HOUR
    }

    #[inline]
    pub fn hours_from_start(hours: f64) -> f64 {
        (START_HOUR + hours) * SECS_PER_HOUR
    }

    #[inline]
    pub fn day_fraction(time: f64) -> f32 {
        ((time % SECS_PER_DAY) / SECS_PER_DAY) as f32
    }

    #[inline]
    pub fn curr_day_fraction(world: &WorldModel) -> f32 {
        Self::day_fraction(world.world_time)
    }

    #[inline]
    pub fn curr_day(world: &WorldModel) -> i32 {
        1 + (world.world_time / SECS_PER_DAY).floor() as i32
    }

    #[inline]
    pub fn curr_day_after_hour(world: &WorldModel, hour: f32) -> i32 {
        1 + ((world.world_time - (hour as f64) * SECS_PER_HOUR) / SECS_PER_DAY).floor() as i32
    }

    #[inline]
    pub fn curr_hour(world: &WorldModel) -> f32 {
        Self::curr_day_fraction(world) * 24.0
    }

    #[inline]
    pub fn curr_hour_or_start(world: &WorldModel) -> f32 {
        (((world.world_time.max(START_WORLD_TIME)) % SECS_PER_DAY) / SECS_PER_HOUR) as f32
    }

    #[inline]
    pub fn curr_time_minutes(world: &WorldModel) -> i32 {
        ((Self::curr_day_fraction(world) * 1440.0) as f64).floor() as i32
    }

    pub fn curr_time_icon(world: &WorldModel) -> TimeOfDayIcon {
        let frac = Self::curr_day_fraction(world) as f64;
        if !(0.2..=0.8).contains(&frac) {
            TimeOfDayIcon::Night
        } else if frac > 0.3 && frac < 0.7 {
            TimeOfDayIcon::Day
        } else if frac > 0.5 {
            TimeOfDayIcon::Dusk
        } else {
            TimeOfDayIcon::Dawn
        }
    }

    pub fn get_hour_after(from_time: f64, full_days: i32, hour: f32) -> f64 {
        let target_frac = hour / 24.0;
        let curr_frac = Self::day_fraction(from_time);
        let delta_days = if curr_frac < target_frac {
            target_frac - curr_frac + full_days as f32
        } else {
            target_frac - curr_frac + full_days as f32 + 1.0
        };
        from_time + (delta_days as f64) * SECS_PER_DAY
    }

    pub fn get_next_hour(world: &WorldModel, hour: f32) -> f64 {
        Self::get_hour_after(world.world_time, 0, hour)
    }

    pub fn get_next_hour_at_least_half_day(world: &WorldModel, hour: f32) -> f64 {
        let mut diff = hour - Self::curr_hour_or_start(world);
        if diff < 0.0 {
            diff += 24.0;
        }
        Self::get_hour_after(world.world_time, if diff < 12.0 { 1 } else { 0 }, hour)
    }

    pub fn get_next_dawn(world: &WorldModel) -> f64 {
        Self::get_hour_after(world.world_time, 0, 6.0)
    }

    pub fn get_next_dawn_after_next_dusk(world: &WorldModel) -> f64 {
        let h = Self::curr_hour(world);
        Self::get_hour_after(world.world_time, if (6.0..=18.0).contains(&h) { 0 } else { 1 }, 6.0)
    }

    /// Port of `TimeDirector.OnPassedHour`.
    pub fn on_passed_hour(world: &WorldModel, hour: f32) -> bool {
        let Some(prev) = world.last_world_time else {
            return false;
        };
        if prev >= world.world_time {
            return false;
        }
        let curr_h = (world.world_time / SECS_PER_HOUR) % 24.0;
        let prev_h = (prev / SECS_PER_HOUR) % 24.0;
        let target = hour as f64;
        (curr_h >= target && prev_h < target)
            || (curr_h < prev_h && (prev_h < target || target <= curr_h))
    }

    /// Port of `TimeDirector.OnPassedTime`.
    pub fn on_passed_time(world: &WorldModel, target_world_time: f64) -> bool {
        let Some(prev) = world.last_world_time else {
            return false;
        };
        prev < target_world_time && target_world_time <= world.world_time
    }

    /// Formats total minutes into `HH:MM` matching `FormatTime`.
    pub fn format_time(total_mins: i32) -> String {
        let h = total_mins / 60;
        let m = total_mins % 60;
        format!("{h:02}:{m:02}")
    }
}
