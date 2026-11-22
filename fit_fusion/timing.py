"""Summarize message timestamps in file order without retaining every sample."""

from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from typing import Optional


FIT_EPOCH = datetime(1989, 12, 31, tzinfo=timezone.utc)


def utc_string(seconds):
    if seconds is None:
        return None
    return (FIT_EPOCH + timedelta(seconds=seconds)).isoformat().replace("+00:00", "Z")


@dataclass
class Timing:
    count: int = 0
    earliest: Optional[int] = None
    latest: Optional[int] = None
    previous: Optional[int] = None
    duplicates: int = 0
    backwards: int = 0
    intervals: Counter = field(default_factory=Counter)

    def observe(self, timestamp):
        if timestamp is None:
            return
        self.count += 1
        self.earliest = timestamp if self.earliest is None else min(self.earliest, timestamp)
        self.latest = timestamp if self.latest is None else max(self.latest, timestamp)
        if self.previous is not None:
            delta = timestamp - self.previous
            if delta > 0:
                self.intervals[delta] += 1
            elif delta == 0:
                self.duplicates += 1
            else:
                self.backwards += 1
        self.previous = timestamp

    def report(self):
        median = None
        n = sum(self.intervals.values())
        # Locate both central observations in the weighted interval histogram.
        if n:
            seen = 0
            middle = []
            for seconds, frequency in sorted(self.intervals.items()):
                for index in ((n - 1) // 2, n // 2):
                    if seen <= index < seen + frequency:
                        middle.append(seconds)
                seen += frequency
            median = sum(middle) / 2

        return {
            "timestamped_count": self.count,
            "start_utc": utc_string(self.earliest),
            "end_utc": utc_string(self.latest),
            "span_s": self.latest - self.earliest if self.count else None,
            "min_interval_s": min(self.intervals, default=None),
            "median_interval_s": median,
            "max_interval_s": max(self.intervals, default=None),
            "typical_hz": 1 / median if median else None,
            "duplicate_intervals": self.duplicates,
            "backward_intervals": self.backwards,
        }
