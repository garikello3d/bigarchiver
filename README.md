## Problem

With this tool you can backup big volumes of data and split it into multiple fixed-size files. Why to split? Because a single huge monolithic file is extremely hard to manage, especially when using network-mounted filesystems (e.g. DavFS, SSHFS, NFS, etc). For most cloud providers, uploading, say, a 500G file is a challenge: it may reject it an with error, or the upload may be interrupted in the middle (with or without error), or any other things may happen depending on the phase of the moon. For example, Google Drive does not work well over DavFS with +2G files, and YandexDisk starts to misbehave around 1G.

The tool compresses the input data stream with XZ algorithm and encrypts using [authenticated encryption](https://en.wikipedia.org/wiki/Authenticated_encryption) providing both confidentiality and integrity. The AES-128-GCM is currently used as it performs super fast on modern CPUs and also gives high resistance. It perfectly fits cloud infrastructures where security is a regulatory requirement. With this type of encryption scheme, any attacker's attempt to modify the encrypted data (without decrypting) will be detected.

Finally, additional assurance is maintained since the integrity of resulting files is verified right after each backup, so one can be sure that when the backups as needed, they are readable and contain the exact source data.

## Usage samples

#### Example to backup data coming from stdin into files

`tar cf - /my/disk | ./bigarchiver backup --buf-size 256 --alg aes128-gcm --auth "My Full Name" --auth-every 32 --pass mysecret --compress-level 6 --split-size 1024 --out-template /path/to/files%%%%%%`

#### Example to restore data from files to stdout:

`./bigarchiver restore --check-free-space /my --buf-size 256 --pass mysecret --config /path/to/files000000.cfg | tar xf - /my/disk`

#### Example to verify the backup files without actual restore:

`./bigarchiver check --buf-size 256 --pass mysecret --config /path/to/files000000.cfg`

#### Example brenchmark different settings and see the performance

`dd if=/dev/urandom bs=1M | ./bigarchiver bench --out-dir /tmp/test --duration 60 --compress-levels 1,3,5,7,9 --buf-sizes 4,32 --compress-threads-nums 1,2,4 --algs none,aes128-gcm`

## Command line option reference

| Option                                                   | Meaning |
|----------------------------------------------------------|---------|
| `backup, restore, check, bench` | select mode of operation (only one at a time) |
| `--alg <alg>` | Encryption & authentication algorithm; possible values: none, aes128-gcm, chacha20-poly1305 |
| `--auth-every <size_mb>` | Embed authentication data to each portion of data of indicated size, in MB |
| `--auth <string>` | Public authentication data to embed |
| `--buf-size <size_mb>` | Buffer size for reading disk files or stdin, in MB |
| `--buf-sizes <size,size,size,...>` | Buffer sizes for reading stdin data to try, comma-separated values (in MB), for benchmarking |
| `--check-free-space <mountpoint_or_path>` | Check free space available on the indicated filesystem before restore |
| `--compress-level <level>` | LZMA compression level, 0 - 9 |
| `--compress-levels <level,level,level,...>` | LZMA compression levels to try, comma-separated levels (0 - 9), for benchmarking |
| `--compress-threads <how_many>` | How many threads to use for compression; defaults to the number of CPU cores if omitted |
| `--compress-threads-nums <n,n,n,...>` | Sequence of numbers of threads to use, comma-separated values, for benchmarking |
| `--config <full_path>` | Full path to config file of the archive to restore |
| `--decompress-threads <how_many>` | How many threads to use for decompression; defaults to the number of CPU cores if omitted |
| `--duration <seconds>` | Limit in seconds for each try, for benchmarking |
| `--no-check` | Do not check the integrity of the whole archive after backup (for backup mode) or before actual restore is done (for restore mode) is done; the default is to always check |
| `--out-dir </path/to/dir>` | Path to directory to store temporary files, for benchmarking |
| `--out-template <path_with_%>` | Template for output chunks; '%' symbols will transform into a sequence number |
| `--pass <password>` | Password to encrypt/decrypt data with |
| `--split-size <size_mb>` | Size of output chunks, in MB |

## Memory usage

The tool allows control of how much memory will be used. On the one hand, the more memory it uses, the faster will be the operation. On the other hand, using too much memory will put other processes' memory pages into swap that may not be desired. So in the absence of one-size-fits-all approach, the option `--buf-size` should be used. The overall memory consumption can be _roughly_ estimated as follows:

`MEM_USAGE_APPRX = BUF_SIZE + XZ_CONSUMPTION`

where _XZ_CONSUMPTION_ is additional memory intensively swallowed by XZ compressor/decompressor module, which, in turn, can be estimated like this:

| XZ level | Compressor,  MB | Decompressor, MB |
|---|---|---|
| 0 | 5 | 1 |
| 1 | 10 | 2 |
| 2 | 20 | 3 |
| 3 | 30 | 5 |
| 4 | 50 | 5 |
| 5 | 100 | 10 |
| 6 | 100 | 10 |
| 7 | 190 | 20 |
| 8 | 370 | 30 |
| 9 | 680 | 65 |

## Q & A

Q: why is this tool needed if one can use something like `tar | xz | openssl | split`?

A: those kind of "shell" approach would require an significant amount of accompanying code, mainly to verify the correctness of the written result. Not to mention the portability problems of different shells in different systems.

Q: why the Authenticated encryption is used, and not just plain old AES (or any other proven symmetric encryption)?

A: basic symmetric encryption provides only confidentiality assurance (meaning unauthorized persons cannot read the data), but it lacks authenticity (meaning no unauthorized modifications can go undetected, even if it's just a dumb corruption of data). This is where AEAD encryption comes into scene.

Q: is the encryption hardware accelerated?

A: yes, as long as your CPU support AES-NI instructions.

Q: which compression level should I use?

A: it depends how much memory and CPU one can devote to backup process. Setting too low levels makes sense when input data is of high randomness (e.g. it already consists of some archive files, so trying to compress them will drain CPU power for nothing), or a machine has very little memory available. Setting too high levels is only useful when the output size is critical and the destination storage is expensive. All in all, for majority of cases levels of 4-6 is the best approach.

Q: if during the backup process something goes wrong, e.g. something cannot be written on the filesystem?

A: the process stops with non-zero exit code leaving everything partially written, i.e. no cleanup is done. Proper cleanup will be probably implemented in the future.

Q: how is the encryption key produced from the string password given?

A: password-based key derivation function PBKDF2-HMAC-SHA256 is used with 100k iterations

## Building for different platforms other than your development host

Sometimes it's needed to quickly check if the binary compiles and passess all tests for some platform, other than the workstation where you cloned repo. Not to mention the binary itself that may not even run on your workstation. In the `build` directory there are scripts to help. They rely on the fact that your platform of interest is either clonable as _docker_ image (linuxes) **or** accessible remotely via _ssh_ (bsd, solaris, aix, ...). Currently a couple of Linuxes and FreeBSD are supported/tested, but you can easily add your own.

Specifically, the steps are:

- change into `scripts` folder
- copy `PLATFORMS.example` to `PLATFORMS`
- review what's inside this file, make necessary changes (e.g. for non-container targets you have to provide a valid hostname/IP)
- run something like `./build.sh --image rocky9`: it will download and build an image with the toolchain
- suppose you made some local changes and want to test them on Rocky Linux before committing anything. In this case you run `./build.sh --app rocky9`: it will grab the local source code, upload it to the spawned container, and build the code + run all the tests
- resulting binary will appear under `build/rocky9` in your main working directory on the host machine

**Note:** running `./build.sh --image`  or `./build.sh --app` (i.e. without specifying the platform identifier) will prepare images or build the app for all platforms listed in `PLATFORMS` file.

## TODO/plans

* proper cleanup after interrupted/failed backup

## Disclaimer

Although the tool is abundant with tests and coded in Rust, it's written by a human and may contain errors. The author has no responsibility on lost data of any production servers in case something goes wrong
