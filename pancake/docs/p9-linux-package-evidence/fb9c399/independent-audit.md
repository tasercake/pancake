# Independent audit of the matched Linux package

The audit verified all 39 package evidence checksums. It streamed the
retained `.deb` through `dpkg-deb --fsys-tarfile` and independently hashed
both executables and the included notice inventory. The digests matched
the package receipt and the executables matched `c1e2cac` byte for byte.
The new source digest binds the qualification adapter, while the case
contract, descriptor, and Cargo and uv locks match the earlier run. The
original exact fast, dependency, and package gate exit statuses were all 0.
[Raw capture identities](source-contract-preflight.json), [package hashes](package-verification.json).

The evidence changes were reapplied as docs-only commits to the sanitized
`86d4df2` history. The exclusion check enumerated reachable Git object ID
strings with replacements and grafts disabled and compared their individual
digests to the excluded ID digest. It did not read the excluded object's
contents. The original source had 7,943 reachable objects and the sorted ID
list matched the established root digest. The sanitized base and both
evidence commits passed the same exclusion check. The 41-file native About
manifest also verified, and its strict comparator retained exit 1.
[Original source ancestry](clean-ancestry.json),
[About comparator](../../p9-cpu-assurance-evidence/fb9c399/native-about-target/live/ordinary-comparator.status).

From the exact suite checkout, use the existing artifact paths and a fresh
output file for an offline receipt verification.
