use std::{cmp, fmt};

use humansize::{BINARY, format_size};
use num_format::{Locale, ToFormattedString};

use crate::{Histogram, Stats};

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "alloc_count: {}",
            self.alloc_count.to_formatted_string(&Locale::en)
        )?;
        writeln!(f, "alloc_sum: {}", format_size(self.alloc_sum, BINARY))?;
        if let Some(alloc_avg) = self.alloc_avg {
            writeln!(f, "alloc_avg: {}", format_size(alloc_avg, BINARY))?;
        }
        writeln!(
            f,
            "\ndealloc_count: {}",
            self.dealloc_count.to_formatted_string(&Locale::en)
        )?;
        writeln!(f, "dealloc_sum: {}", format_size(self.dealloc_sum, BINARY))?;
        if let Some(dealloc_avg) = self.dealloc_avg {
            writeln!(f, "dealloc_avg: {}", format_size(dealloc_avg, BINARY))?;
        }
        writeln!(
            f,
            "\nrealloc_growth_count: {}",
            self.realloc_growth_count.to_formatted_string(&Locale::en)
        )?;
        writeln!(
            f,
            "realloc_growth_sum: {}",
            format_size(self.realloc_growth_sum, BINARY)
        )?;
        if let Some(realloc_growth_avg) = self.realloc_growth_avg {
            writeln!(
                f,
                "realloc_growth_avg: {}",
                format_size(realloc_growth_avg, BINARY)
            )?;
        }
        writeln!(
            f,
            "\nrealloc_shrink_count: {}",
            self.realloc_shrink_count.to_formatted_string(&Locale::en)
        )?;
        writeln!(
            f,
            "realloc_shrink_sum: {}",
            format_size(self.realloc_shrink_sum, BINARY)
        )?;
        if let Some(realloc_shrink_avg) = self.realloc_shrink_avg {
            writeln!(
                f,
                "realloc_shrink_avg: {}",
                format_size(realloc_shrink_avg, BINARY)
            )?;
        }
        writeln!(
            f,
            "\nrealloc_move_count: {}",
            self.realloc_move_count.to_formatted_string(&Locale::en)
        )?;
        writeln!(
            f,
            "realloc_move_sum: {}",
            format_size(self.realloc_move_sum, BINARY)
        )?;
        if let Some(realloc_move_avg) = self.realloc_move_avg {
            writeln!(
                f,
                "realloc_move_avg: {}",
                format_size(realloc_move_avg, BINARY)
            )?;
        }
        writeln!(f, "\nuse_curr: {}", format_size(self.use_curr, BINARY))?;
        writeln!(f, "use_max: {}", format_size(self.use_max, BINARY))?;

        writeln!(f, "\nalloc_buckets:\n{}", &self.alloc_buckets)?;
        writeln!(
            f,
            "realloc_growth_buckets:\n{}",
            &self.realloc_growth_buckets
        )?;
        writeln!(
            f,
            "realloc_shrink_buckets:\n{}",
            &self.realloc_shrink_buckets
        )?;

        Ok(())
    }
}

impl fmt::Display for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const BAR_WIDTH: usize = 40;

        let Some(first) = self.buckets.iter().position(|&c| c > 0) else {
            writeln!(f, "(empty)")?;
            return Ok(());
        };
        let last = self.buckets.iter().rposition(|&c| c > 0).unwrap();
        let max = *self.buckets[first..=last].iter().max().unwrap();

        for (k, &count) in (first..=last).zip(self.buckets[first..=last].iter()) {
            let lo = 1usize << k;
            // bucket k covers [2^k, 2^(k+1) - 1]; the top bucket has no finite upper bound
            let lo_str = format_size(lo, BINARY);
            let range_str = match lo.checked_shl(1) {
                Some(hi) => format!("[{:>9} .. {:>9})", lo_str, format_size(hi, BINARY)),
                None => format!("[{:>9} ..       inf)", lo_str),
            };
            write!(
                f,
                "{}: {:>12}  ",
                range_str,
                count.to_formatted_string(&Locale::en),
            )?;
            // scale bar to max in the trimmed range; show a thin bar for any non-zero
            // count so tiny buckets don't disappear entirely
            let bar_len = if count == 0 {
                0
            } else {
                cmp::max(1, count.saturating_mul(BAR_WIDTH) / max)
            };
            for _ in 0..bar_len {
                f.write_str("█")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
