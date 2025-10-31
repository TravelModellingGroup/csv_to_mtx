# csv_to_mtx

This console program is designed to convert CSV files to mtx/ mtx.gz files. This is typically done when a person wants to manually edited the values and then needs to convert it back into a matrix.

`csv_to_mtx` is the mirror program for [mtx_to_csv](https://github.com/TravelModellingGroup/mtx_to_csv).

```cmd
Usage: csv_to_mtx <input.csv> <output.mtx/.mtx.gz> [zones.csv]"
```

The third parameter is optional, allowing you to specify the shape of the zone system using an additional CSV file.  The `zones.csv` file is expected to be a CSV with 

## Compiling

csv_to_mtx compiles with Rust's Cargo system.

```cmd
cargo build --release
```

## Running the program

Running the code is also possible from Cargo. Doing so without any parameters will give you a prompt of how to use the program.

```cmd
cargo run --release;
```
To convert a csv you can run it from `cargo` using the following command:

```cmd
cargo run --release -- <PATH TO YOUR CSV> <OUTPUT FILE PATH [.mtx/.mtx.gz]>
```

Use the `.mtx.gz` to save it as a compressed matrix, which is supported in `XTMF 1.8+`.

`csv_to_mtx` assumes that if the CSV has more then 3 columns it is reading in a `Square CSV`.  If
it does have exactly three columns it will expect a file in the `Column CSV` format.

### Square CSV

Here is a simple example of a 3x3 matrix where the TAZ are 1,2,3 with some sample data.

|Origin\Destination|	1|	2|	3|
-------------------|-----|----|---|
|1	|0.1	|0.2	|0.3|
|2	|0.4	|0.5	|0.6|
|3	|0.7	|0.8	|0.9|


### Column CSV

Here is the same example as a column based CSV.

|Origin |Destination |Value|
|-------|------------|-----|
|1|1|0.1|
|1|2|0.2|
|1|3|0.3|
|2|1|0.4|
|2|2|0.5|
|2|3|0.6|
|3|1|0.7|
|3|2|0.8|
|3|3|0.9|
