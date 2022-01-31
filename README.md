# Building

Run `cargo run`, for release spec use `cargo run --release`.

# Adding a track

The process I have been following so far:

1. Add Blender to the path variable
2. Install protobuf as a Blender module: Navigate to `C:\Program Files\Blender Foundation\Blender {v}\{v}\python\bin` and run the following commands (you probably have to run powershell as administrator):
   ```
   .\python.exe -m ensurepip
   .\python.exe -m pip install -U pip
   .\python.exe -m pip install protobuf
   ```
3. Open blender and in a single Bezier-curve create the track. Make sure
   to start and end at the track finish line.
4. Generate Python protobuf:
   ```
   mkdir scripts/build
   protoc --proto_path=src --python_out=scripts/build src/track.proto
   ```
4. Run the following to generate the map description (assuming the blender file 
   is called `[...]\hungaroring\grandprix.blend`:
   ```
   blender --b .\media\tracks\hungaroring\grandprix.blend -P .\scripts\blender.py
   ```
5. Move the resulting `output.dat` file to the desired location. In this case
   `media/tracks/hungaroring/grandprix.dat`.
