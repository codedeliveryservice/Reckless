PARAMETERS = "./parameters.rs"

rust_to_ob = {
    "i32": "int",
    "usize": "int",
    "f32": "float",
    "f64": "float",
}

with open(PARAMETERS, "r") as file:
    for line in file.readlines():
        if line.strip().startswith(tuple(rust_to_ob.keys())):
            line = line.replace(";", "").replace(":", "")
            type_, name, default = line.split()

            cpp_type = rust_to_ob[type_]
            min_val = 1
            max_val = 2.0 * float(default)
            step = round((max_val - min_val) / 20, 3)

            print(f"{name}, {cpp_type}, {default}, {min_val}, {max_val}, {step}, 0.002")
