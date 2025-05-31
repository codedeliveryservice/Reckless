PARAMETERS = ".\parameters.rs"

file = open(PARAMETERS, "r")

for line in file.readlines():
    if line.strip().startswith(("i32", "usize", "f32", "f64")):
        (type, name, default) = line.replace(";", "").replace(":", "").split()

        min = 1
        max = 2.0 * float(default)
        step = round((max - min) / 15, 3)

        print(f"{name}, {type}, {default}, {min}, {max}, {step}, 0.002")
