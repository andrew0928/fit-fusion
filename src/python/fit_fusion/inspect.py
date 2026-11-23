"""Inventory the encoded fields and observed sampling of each FIT message type."""

from dataclasses import dataclass, field

from .timing import Timing


def has_value(value):
    if isinstance(value, (tuple, list)):
        return any(item is not None for item in value)
    return value is not None


@dataclass(eq=False)
class FieldStats:
    metadata: dict
    sizes: set = field(default_factory=set)
    samples: int = 0
    timing: Timing = field(default_factory=Timing)

    def report(self, rows):
        return {
            **self.metadata,
            "encoded_sizes_bytes": sorted(self.sizes),
            "sample_count": self.samples,
            "coverage_pct": 100 * self.samples / rows if rows else None,
            "timing": self.timing.report(),
        }


@dataclass
class MessageStats:
    name: str
    unknown: bool
    rows: int = 0
    fields: dict = field(default_factory=dict)
    timing: Timing = field(default_factory=Timing)

    def register(self, definition):
        index = getattr(definition, "dev_data_index", None)
        number = definition.def_num
        name = definition.name or f"unknown_{number}"
        schema = definition.field
        units = getattr(schema, "units", None)
        return self.register_field(index, number, name, definition.base_type.name, units,
                                   schema is None or not schema.name, definition.size)

    def register_field(self, index, number, name, base_type, units, unknown, size):
        key = (index, number, name, base_type, units)
        if key not in self.fields:
            self.fields[key] = FieldStats({
                "number": number,
                "name": name,
                "base_type": base_type,
                "units": units,
                "developer_data_index": index,
                "unknown": unknown,
            })
        self.fields[key].sizes.add(size)
        return self.fields[key]

    def observe(self, frame):
        self.rows += 1
        timestamp = None
        for item in frame.fields:
            # 253 is the FIT timestamp number, including compressed headers.
            # A developer field can independently use the same number.
            if item.def_num == 253 and not getattr(item.field_def, "is_dev", False):
                raw = item.raw_value
                if isinstance(raw, int) and 0x10000000 <= raw < 0xFFFFFFFF:
                    timestamp = raw
                    break
        self.timing.observe(timestamp)

        populated = set()
        for item in frame.fields:
            if item.field_def is None or not has_value(item.raw_value):
                continue
            stats = self.register(item.field_def)
            # Count containing messages, even if a definition repeats a field.
            if stats not in populated:
                stats.samples += 1
                stats.timing.observe(timestamp)
                populated.add(stats)

    def report(self, number):
        return {
            "number": number,
            "name": self.name,
            "unknown": self.unknown,
            "message_count": self.rows,
            "timing": self.timing.report(),
            "fields": [stats.report(self.rows) for stats in self.fields.values()],
        }


def inspect_file(path):
    import fitdecode

    members = []
    with path.open("rb") as source, fitdecode.FitReader(
        source, check_crc=fitdecode.CrcCheck.RAISE
    ) as reader:
        for frame in reader:
            if isinstance(frame, fitdecode.FitHeader):
                header = frame
                messages = {}
            elif isinstance(frame, fitdecode.FitDefinitionMessage):
                number = frame.global_mesg_num
                if number not in messages:
                    messages[number] = MessageStats(frame.name, frame.mesg_type is None)
                for definition in frame.all_field_defs:
                    messages[number].register(definition)
            elif isinstance(frame, fitdecode.FitDataMessage):
                messages[frame.def_mesg.global_mesg_num].observe(frame)
            elif isinstance(frame, fitdecode.FitCRC):
                members.append({
                    "protocol_version": ".".join(map(str, header.proto_ver)),
                    "profile_version": "{}.{:02}".format(*header.profile_ver),
                    "messages": [messages[n].report(n) for n in sorted(messages)],
                })
    if not members:
        raise ValueError("file contains no complete FIT member")
    return {"file": str(path), "members": members}
