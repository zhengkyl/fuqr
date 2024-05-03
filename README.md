# fuqr

a big fu to qr codes

## evidence board

- eci (needs investigation)

- mirror and invert should be handled by frontend

- ~~structured append~~

  - not enough decoder supported (eg google lens)

- e + 2t <= d - p

  - _e_ erasure
  - _t_ error
  - _d_ correction codewords
  - _p_ misdecode protection codewords

- avoid 1011101 (finder pattern) as much as possible

- version (< 7) based on dist between finder patterns (reference algo)

- why does Annex A contain generator polynomials for up to 68 error correction codewords?

  - 30 is the max possible, b/c 15 correctable errors per block

- burger king commercial

## references

https://www.thonky.com/qr-code-tutorial/

https://github.com/erwanvivien/fast_qr

https://github.com/zxing/zxing

https://github.com/antfu/qrcode-toolkit

https://github.com/unjs/uqr

https://web.archive.org/web/20150321031237/http://research.swtch.com/qart

https://web.archive.org/web/20150321025905/http://research.swtch.com/field
