# JCS reference fixtures

These input and output pairs come from `testdata/{input,output}` in
[cyberphone/json-canonicalization](https://github.com/cyberphone/json-canonicalization)
at commit `19d51d7fe467d4706a3ff08adf8a748f29fc21e0`. The original notice is
in [LICENSE](LICENSE).

The `values.json` case includes `1E30`, which Codec rejects under its integer
range policy.
