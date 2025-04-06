# One web satelitte Timepix data processor

## Usage

```bash
Convertor of oneweb timepix data

Usage: one-web-extractor --gps-file <GPS_FILE> --meas-file <MEAS_FILE> --data-file <DATA_FILE> --output-directory <OUTPUT_DIRECTORY>

Options:
  -g, --gps-file <GPS_FILE>                  Path to gps file (dosimeter_gps_info.csv)
  -m, --meas-file <MEAS_FILE>                Path to measurement file (dosimeter_measure_info.csv)
  -d, --data-file <DATA_FILE>                Path to data file (dosimeter_image_packets.csv)
  -o, --output-directory <OUTPUT_DIRECTORY>  Output directory
  -h, --help                                 Print help
  -V, --version                              Print version
```

## Example

```powershell
one-web-extractor.exe -g data/dosimeter_gps_info.csv -m data/dosimeter_measure_info.csv -d data/dosimeter_image_packets.csv -o output
```