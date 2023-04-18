use std::str::FromStr;

use chrono::{DateTime, Local};
use serde::Deserialize;
use uuid::Uuid;
use cron::Schedule as CronSchedule;

#[derive(Debug)]
pub struct Schedule {
    pub name: String,
    pub playlist: Uuid,
    scheduled: Scheduled
}

impl<'de> Deserialize<'de> for Schedule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let input = ScheduleInput::deserialize(deserializer).unwrap();
        Ok(Schedule {
            name: input.name,
            playlist: input.playlist,
            scheduled: Scheduled::new(input.scheduled.unwrap_or(vec![]), input.playlist)
        })
    }
}

#[derive(Deserialize, Debug)]
struct ScheduleInput {
    name: String,
    scheduled: Option<Vec<ScheduledPlaylistInput>>,
    playlist: Uuid
}

#[derive(Deserialize, Debug)]
struct ScheduledPlaylistInput {
    playlist: Uuid,
    start: String,
    end: String
}

#[derive(Debug)]
enum ScheduledItem {
    Schedule {
        start: CronSchedule,
        end: CronSchedule,
        playlist: Uuid
    },
    Fallback (Uuid)
}

#[derive(Debug)]
struct Scheduled {
    schedules: Vec<ScheduledItem>
}

impl Scheduled {
    fn new(playlist: Vec<ScheduledPlaylistInput>, fallback: Uuid) -> Self {
        Scheduled { 
            schedules: 
                playlist.iter().map(|s| ScheduledItem::Schedule { 
                    start: CronSchedule::from_str(&s.start).unwrap(),
                    end: CronSchedule::from_str(&s.end).unwrap(),
                    playlist: s.playlist
                })
                .chain(vec![ScheduledItem::Fallback(fallback)].into_iter())
                .collect()
        }
    }

    /// Provides last scheduled time from given `DateTime` including the current time if applicable.
    fn previous_time(from: &DateTime<Local>, schedule: &CronSchedule) -> Option<DateTime<Local>> {
        match schedule.after(from).next_back() {
            // Check if current time is a scheduled moment
            _ if schedule.includes(from.clone()) => Some(from.clone()),
            // Check last schedules moment if any
            Some(back) if &back <= from => Some(back),
            // No scheduled moments have existed before
            _ => None
        }
    }

    /// Get the current active playlist in schedule at the provided point in time
    pub fn current_playlist(&self, time: &DateTime<Local>) -> Uuid {
        self.schedules.iter().find_map(|schedule| {
            // A branch returning a Some signals last scheduled moment was a start action, None is used otherwise.
            // find_map returns the value of the first closure to return a Some, and since all Vectors in Scheduled
            // include a fallback resulting in a guaranteed Some, unwrap may safely be used to avoid returning an Option
            match schedule {
                ScheduledItem::Schedule { start, end, playlist } => {
                    let (last_start, last_end) =
                        (Self::previous_time(time, start), Self::previous_time(time, end));
                    
                    if let (Some(last_start), Some(last_end)) = (last_start, last_end) {
                        // If both previous start and end moments were found, return Some if the most recent one was a start action
                        match last_start.cmp(&last_end) {
                            std::cmp::Ordering::Less => None,
                            _ => Some(playlist.clone()),
                        }
                    } else {
                        // If at most one previous moment was found, return Some if it was a last_start
                        last_start.and_then(|_| Some(playlist.clone()))
                    }
                },
                ScheduledItem::Fallback(uuid) => Some(uuid.clone()),
            }
        }).unwrap()
    }
}

#[cfg(test)]
mod test {
    use chrono::{Local, TimeZone};
    use uuid::Uuid;

    use super::{Scheduled, ScheduledPlaylistInput};

    #[test]
    fn test_current_playlist_specific_date() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let playlist = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 18 4 * 2023".to_string(),
                end: "0 0 14 18 4 * 2023".to_string(),
            }
        ];
        let schedule: Scheduled = Scheduled::new(playlist, default_uuid);

        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap()));

        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap()));
    }

    #[test]
    fn test_current_playlist() {
        let scheduled_uuid = Uuid::parse_str("8626f6e1-df7c-48d9-83c8-d7845b774ecd").unwrap();
        let default_uuid = Uuid::parse_str("25cd63df-1f10-4c3f-afdb-58156ca47ebd").unwrap();
        let playlist = vec![
            ScheduledPlaylistInput {
                playlist: scheduled_uuid,
                start: "0 0 10 * * * *".to_string(),
                end: "0 0 14 * * * *".to_string(),
            }
        ];
        let schedule: Scheduled = Scheduled::new(playlist, default_uuid);

        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 9, 59, 59).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 0).unwrap()));
        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 10, 0, 1).unwrap()));

        assert_eq!(scheduled_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 13, 59, 59).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 0).unwrap()));
        assert_eq!(default_uuid, schedule.current_playlist(&Local.with_ymd_and_hms(2023, 4, 18, 14, 0, 1).unwrap()));
    }
}
