# vfsops

Dump out vfs op buckets for all running zones.

The first output shows historic "since boot" data, while the following output
shows you the difference since the last interval (currently hardcoded at 5s).

### Usage

```
vfsops 0.1.0
Mike Zeller <mike@mikezeller.net

The vfsops utility reports vfs operation outliers. A count of the number of operations are grouped
into 10ms, 100ms, 1s, and 10s buckets. The first line of output represents a zone's outliers since
boot, while sequential lines show how many operations have occured during the INTERVAL. By default
vfsops will only output zones that have a non zero value for all buckets.


USAGE:
    vfsops [FLAGS] [OPTIONS] <INTERVAL> [COUNT]

FLAGS:
    -H               Don't print the header
    -Z               Print zones with no activity
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -z, --zone <ZONE>    Print data for a specific zonename

ARGS:
    <INTERVAL>    Print results per inverval rather than per second
    <COUNT>       Print for n times and exit
```

### Example

```
# vfsops -Z 1 5
    zone   10ms_ops  100ms_ops     1s_ops    10s_ops
61ff1cb8       3349        111          0          0
61ff1cb8          0          0          0          0
61ff1cb8          0          0          0          0
61ff1cb8          0          0          0          0
61ff1cb8          0          0          0          0
```
