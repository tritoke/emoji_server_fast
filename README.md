# emoji_server_fast
Solution to the emoji_server challenge from pwned3, in rust, because why not?

## speed?
I tried to make it a fast as I could?

It uses a [`SIMD`](https://en.wikipedia.org/wiki/Single_instruction,_multiple_data) accelerated
[levenshtein distance](https://github.com/Daniel-Liu-c0deb0t/triple_accel) algorithm, it precalculates a cache of the
levenshtein distance of all possible combinations of words
and it uses [`rayon`](https://docs.rs/rayon/latest/rayon/) to parallelise many expensive operations, such as populating the cache etc.

Overall it can get the flag, on average in less than a minute.
