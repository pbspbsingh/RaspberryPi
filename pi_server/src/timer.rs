use std::time::{Duration, Instant};

pub trait Timer {
    fn t(&self) -> String;
}

impl Timer for Instant {
    fn t(&self) -> String {
        self.elapsed().t()
    }
}

impl Timer for Duration {
    #[inline]
    fn t(&self) -> String {
        let ms = self.as_millis() as u64;
        let secs = ms / 1000;
        let minutes = secs / 60;
        let hours = minutes / 60;
        let days = hours / 24;
        if days > 0 {
            format!(
                "{}days {}hours {}mins",
                days,
                hours - days * 24,
                minutes - hours * 60,
            )
        } else if hours > 0 {
            format!(
                "{}hours {}mins {}secs",
                hours,
                minutes - hours * 60,
                secs - minutes * 60
            )
        } else if minutes > 0 {
            format!("{}mins {}secs", minutes, secs - minutes * 60)
        } else if secs > 0 {
            let sms = ms % 1000;
            if sms >= 100 {
                format!("{}.{}secs", secs, sms / 100)
            } else {
                format!("{}secs", secs)
            }
        } else {
            format!("{}ms", ms)
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::Timer;

    #[test]
    fn test() {
        let hehe = [100, 9500, 10000, 1000000, 123402312];
        for s in &hehe {
            println!("{}", Duration::from_millis(*s).t());
        }
    }
}
