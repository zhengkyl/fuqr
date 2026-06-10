# FVQR

Generate valid QR codes from fixed pixels and error locations.

## Examples


## Background

### QR Art

QR codes use Reed-Solomon (RS) error correction. The black and white pixels represent data, consisting of `k` message symbols and `r` derived error correction symbols. Changing the pixels requires changing the message symbols, but changing the message defeats the purpose. However if the message contents have length `c` shorter than `k`, then `p` padding symbols are added such that `c + p = k`. These padding symbols can be arbitrarily set in practice.

It is straightforward to control the pixels corresponding to `p` padding symbols, but actually we can choose any mixture of `p` padding or error correction symbols, and solve for the required unknown padding symbols. In short, RS codes are [Maximum Distance Separable](https://en.wikipedia.org/wiki/Singleton_bound#MDS_codes) and for any arbitrary `k` chosen symbols, there is exactly one matching RS code.

In addition, standard QR code decoding can correct up to `⌊r / 2⌋` incorrect symbols at unknown locations. This allows placing pixels anywhere, but since it is half as efficient as the "solve for padding" method above, this should only be used for forcing pixels corresponding to the message content or when more than `p` symbols need to be forced.

Here are the earliest descriptions of similar ideas I found online, although likely independently identified many times. The basic idea is explained on the [Reed-Solomon Wikipedia page](https://en.wikipedia.org/wiki/Reed%E2%80%93Solomon_error_correction#Systematic_encoding_procedure:_The_message_as_an_initial_sequence_of_values).

- [Expansion of Image Displayable Area in Design QR Code and Its Applications, 2011](https://www.ieice.org/publications/conferences/summary.php?id=FIT0000009019&expandable=2&ConfCd=F&session_num=4V&lecture_number=O-006&year=2011&conf_type=F)

- [QArt codes, 2012](https://research.swtch.com/qart)

## Undocumented knowledge

- Byte mode is UTF-8.

- The data mask scoring step is unnecessary. The best mask is unambiguously `2` (vertical stripes), but even using the worst mask, `0` (checkboard), has no discernible impact on scanability.

- When calculating error correction capacity, there is no need to account for misdecode protection for Version 1 Low, Medium and Version 2 Low.

- The timing patterns are not used. Only the bottom right alignment pattern is used although scanning can succeed without it, especially on smaller codes.