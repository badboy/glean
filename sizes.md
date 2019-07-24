# Compiled code size

## General config

Versions:

```
$ rustc --version
rustc 1.36.0 (a53f9df32 2019-07-03)

$ rustc +nightly --version
rustc 1.37.0-nightly (0af8e872e 2019-06-30)
```

`.cargo/config`

```toml
[target.aarch64-linux-android]
ar = "aarch64-linux-android-ar"
linker = "aarch64-linux-android-clang"

[target.armv7-linux-androideabi]
ar = "arm-linux-androideabi-ar"
linker = "arm-linux-androideabi-clang"

[target.i686-linux-android]
ar = "i686-linux-android-ar"
linker = "i686-linux-android-clang"

[target.x86_64-linux-android]
ar = "x86_64-linux-android-ar"
linker = "x86_64-linux-android-clang"
```

All binaries are located in `~/Library/Android/rust-android-ndk-toolchains/`, installed by the `rust-android-gradle` plugin.

## Default `cargo` configuration

`Cargo.toml`:

*no `profile.*` settings*

| Target | Build type | Size in bytes | Size in megabytes |
|---|---|---|---|
| aarch64-linux-android | debug | 39034792 | 37.226479 |
| aarch64-linux-android | release | 5221144 | 4.9792709 |
| aarch64-linux-android | release (stripped) | 2070512 | 1.9745941 |
| armv7-linux-androideabi | debug | 36865756 | 35.157925 |
| armv7-linux-androideabi | release | 4274756 | 4.076725 |
| armv7-linux-androideabi | release (stripped) | 1439000 | 1.3723373 |
| i686-linux-android | debug | 40409540 | 38.53754 |
| i686-linux-android | release | 5035832 | 4.8025436 |
| i686-linux-android | release (stripped) | 2418076 | 2.306057 |
| x86_64-linux-android | debug | 39219248 | 37.40239 |
| x86_64-linux-android | release | 5304856 | 5.0591049 |
| x86_64-linux-android | release (stripped) | 2447544 | 2.3341599 |

## Current configuration

`Cargo.toml`:

```toml
[profile.release]
opt-level = "s"
debug = true
lto = true
```

| Target | Build type | Size in bytes | Size in megabytes |
|---|---|---|---|
| aarch64-linux-android | release | 32607296 | 31.096741 |
| aarch64-linux-android | release (stripped) | 2009072 | 1.9160004 |
| armv7-linux-androideabi | release | 30429032 | 29.019386 |
| armv7-linux-androideabi | release (stripped) | 1340696 | 1.2785873 |
| i686-linux-android | release | 30232488 | 28.831947 |
| i686-linux-android | release (stripped) | 2266524 | 2.1615257 |
| x86_64-linux-android | release | 33275672 | 31.734154 |
| x86_64-linux-android | release (stripped) | 2328760 | 2.2208786 |

## Nightly compiler (1.37-nightly, 2019-06-30)

`Cargo.toml`:

*no `profile.*` settings*

| Target | Build type | Size in bytes | Size in megabytes |
|---|---|---|---|
| aarch64-linux-android | debug | 39253112 | 37.434685 |
| aarch64-linux-android | release | 5135040 | 4.8971558 |
| aarch64-linux-android | release (stripped) | 2078656 | 1.9823608 |
| armv7-linux-androideabi | debug | 37348108 | 35.617931 |
| armv7-linux-androideabi | release | 4218180 | 4.0227699 |
| armv7-linux-androideabi | release (stripped) | 1451264 | 1.3840332 |
| i686-linux-android | debug | 40924576 | 39.028717 |
| i686-linux-android | release | 5021452 | 4.7888298 |
| i686-linux-android | release (stripped) | 2446748 | 2.3334007 |
| x86_64-linux-android | debug | 39495968 | 37.66629 |
| x86_64-linux-android | release | 5248856 | 5.0056992 |
| x86_64-linux-android | release (stripped) | 2467960 | 2.3536301 |

## `panic=abort`

`Cargo.toml`:

```toml
[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

| Target | Build type | Size in bytes | Size in megabytes |
|---|---|---|---|
| aarch64-linux-android | debug | 37969032 | 36.210091 |
| aarch64-linux-android | release | 4838880 | 4.6147156 |
| aarch64-linux-android | release (stripped) | 1832944 | 1.7480316 |
| armv7-linux-androideabi | debug | 36174012 | 34.498226 |
| armv7-linux-androideabi | release | 4014748 | 3.8287621 |
| armv7-linux-androideabi | release (stripped) | 1295640 | 1.2356186 |
| i686-linux-android | debug | 39417984 | 37.591919 |
| i686-linux-android | release | 4603888 | 4.3906097 |
| i686-linux-android | release (stripped) | 2090396 | 1.993557 |
| x86_64-linux-android | debug | 38185560 | 36.416588 |
| x86_64-linux-android | release | 4917040 | 4.6892548 |
| x86_64-linux-android | release (stripped) | 2181304 | 2.0802536 |

## xargo on Rust nightly

`Xargo.toml`:

```toml
[target.aarch64-linux-android.dependencies.std]
features = []

[target.armv7-linux-androideabi.dependencies.std]
features = []

[target.i686-linux-android.dependencies.std]
features = []

[target.x86_64-linux-android.dependencies.std]
features = []
```

`Cargo.toml`:

```toml
[profile.release]
panic = "abort"
```

| Target | Build type | Size in bytes | Size in megabytes |
|---|---|---|---|
| aarch64-linux-android | release | 2342480 | 2.233963 |
| aarch64-linux-android | release (stripped) | 1754976 | 1.6736755 |
| armv7-linux-androideabi | release | 1711544 | 1.6322556 |
| armv7-linux-androideabi | release (stripped) | 1230072 | 1.1730881 |
| i686-linux-android | release | 2385920 | 2.2753906 |
| i686-linux-android | release (stripped) | 2004324 | 1.9114723 |
| x86_64-linux-android | release | 2514888 | 2.3983841 |
| x86_64-linux-android | release (stripped) | 2091048 | 1.9941788 |

## xargo with LTO on Rust nightly

`Xargo.toml`:

```toml
[target.aarch64-linux-android.dependencies.std]
features = []

[target.armv7-linux-androideabi.dependencies.std]
features = []

[target.i686-linux-android.dependencies.std]
features = []

[target.x86_64-linux-android.dependencies.std]
features = []
```

`Cargo.toml`:

```toml
[profile.release]
panic = "abort"
lto = true
```

| Target | Build type | Size in bytes | Size in megabytes |
|---|---|---|---|
| aarch64-linux-android | release | 2343216 | 2.2346649 |
| aarch64-linux-android | release (stripped) | 1754976 | 1.6736755 |
| armv7-linux-androideabi | release | 1712400 | 1.6330719 |
| armv7-linux-androideabi | release (stripped) | 1230072 | 1.1730881 |
| i686-linux-android | release | 2390532 | 2.279789 |
| i686-linux-android | release (stripped) | 2008420 | 1.9153786 |
| x86_64-linux-android | release | 2519592 | 2.4028702 |
| x86_64-linux-android | release (stripped) | 2095144 | 1.998085 |

## application-services

Default release build with and without Glean.

| Target | Build type | Size in bytes | Size in megabytes |
|---|---|---|---|
| aarch64-linux-android | master | 7746440 | 7.3875809 |
| aarch64-linux-android | with-glean | 8484800 | 8.091758 |
| armv7-linux-androideabi | master | 6897480 | 6.5779495 |
| armv7-linux-androideabi | with-glean | 7819184 | 7.456955 |
