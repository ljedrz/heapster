use alloc::format;
use core::{cmp, fmt};

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

        if self.realloc_growth_count > 0 {
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
        }

        if self.realloc_shrink_count > 0 {
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
        }

        if self.realloc_move_count > 0 {
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
        }

        if self.alloc_fail_count > 0 {
            writeln!(
                f,
                "\nalloc_fail_count: {}",
                self.alloc_fail_count.to_formatted_string(&Locale::en)
            )?;
        }
        if self.realloc_fail_count > 0 {
            writeln!(
                f,
                "realloc_fail_count: {}",
                self.realloc_fail_count.to_formatted_string(&Locale::en)
            )?;
        }

        writeln!(f, "\nuse_curr: {}", format_size(self.use_curr, BINARY))?;
        writeln!(f, "use_max: {}", format_size(self.use_max, BINARY))?;

        if self.alloc_histogram.total() > 0 {
            writeln!(f, "\nalloc_histogram:\n{}", &self.alloc_histogram)?;
        }

        if self.realloc_growth_histogram.total() > 0 {
            writeln!(
                f,
                "realloc_growth_histogram:\n{}",
                &self.realloc_growth_histogram
            )?;
        }

        if self.realloc_shrink_histogram.total() > 0 {
            writeln!(
                f,
                "realloc_shrink_histogram:\n{}",
                &self.realloc_shrink_histogram
            )?;
        }

        Ok(())
    }
}

impl fmt::Display for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const BAR_WIDTH: u128 = 40;

        let Some(first) = self.buckets().iter().position(|&c| c > 0) else {
            writeln!(f, "(empty)")?;
            return Ok(());
        };
        let last = self.buckets().iter().rposition(|&c| c > 0).unwrap();
        let max = *self.buckets()[first..=last].iter().max().unwrap();

        // widths scale with content: the count column tracks the largest
        // formatted count in the displayed range; the size column is the
        // tightest fixed width that fits every power-of-two label.
        let size_width = 7usize;
        let count_width = max.to_formatted_string(&Locale::en).len();

        for (k, &count) in (first..=last).zip(self.buckets()[first..=last].iter()) {
            let lo = 1usize << k;
            let lo_str = format_size(lo, BINARY);
            // bucket k covers [2^k, 2^(k+1) - 1]; the top bucket has no finite upper bound
            let range_str = match lo.checked_mul(2) {
                Some(hi) => format!(
                    "[{:>size_width$} .. {:>size_width$})",
                    lo_str,
                    format_size(hi, BINARY),
                ),
                None => format!("[{:>size_width$} .. {:>size_width$})", lo_str, "inf"),
            };
            write!(
                f,
                "{}: {:>count_width$}  ",
                range_str,
                count.to_formatted_string(&Locale::en),
            )?;
            let bar_len = if count == 0 {
                0
            } else {
                cmp::max(
                    1,
                    ((count as u128).saturating_mul(BAR_WIDTH) / max as u128) as usize,
                )
            };
            for _ in 0..bar_len {
                f.write_str("█")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
