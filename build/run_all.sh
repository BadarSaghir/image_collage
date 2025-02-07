#!/bin/bash
# run_all.sh

# Define common parameters
INPUT_DIR="./posts"      # Change this to your actual input directory
CELL_SIZE=350                       # Change cell size if needed

# Define output files for each executable run
OUTPUT1="output1.webp"
OUTPUT2="output2.webp"
OUTPUT3="output3.webp"

# Define log files for resource usage output
LOG1="go_collage.log"
LOG2="py_college.log"
LOG3="rust_collage.log"

# Run executable 1
echo "Running exec1..."
/usr/bin/time -v ./go_collage -input_dir "$INPUT_DIR" -output_file "$OUTPUT1" --cell_size "$CELL_SIZE" 2> "$LOG1"
echo "Output saved to $OUTPUT1, log saved to $LOG1"
echo

# Run executable 2
echo "Running exec2..."
/usr/bin/time -v ./py_college -input_dir "$INPUT_DIR" -output_file "$OUTPUT2" --cell_size "$CELL_SIZE" 2> "$LOG2"
echo "Output saved to $OUTPUT2, log saved to $LOG2"
echo

# Run executable 3
echo "Running exec3..."
/usr/bin/time -v ./rust_collage -input_dir "$INPUT_DIR" -output_file "$OUTPUT3" --cell_size "$CELL_SIZE" 2> "$LOG3"
echo "Output saved to $OUTPUT3, log saved to $LOG3"
echo "All done."
