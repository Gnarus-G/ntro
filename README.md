[![crates.io](https://img.shields.io/crates/v/ntro.svg)](https://crates.io/crates/ntro)
[![npm version](https://img.shields.io/npm/v/ntro.svg)](https://www.npmjs.com/package/ntro)

# ntro

A cli tool that to ntrospect configuration files and output a typescript type declarations.

## Features

### .yaml -> .d.ts

```yaml
# filename: test.multiple.yaml
calling-birds:
  - huey
  - dewey
  - louie
  - fred
doe: "a deer, a female deer"
french-hens: 3
pi: 3.14159
ray: "a drop of golden sun"
xmas: true
xmas-fifth-day:
  calling-birds: four
  french-hens: 3
  golden-rings: 5
  partridges:
    count: 1
    location: "a pear tree"
  turtle-doves: two
---
calling-birds:
  - huey
  - dewey
  - louie
  - fred
doe: "a deer, a female deer"
---
hello: world
```

becomes

```ts
declare namespace TestMultiple {
  export type Document0 = {
    "calling-birds": ["huey", "dewey", "louie", "fred"];
    doe: "a deer, a female deer";
    "french-hens": 3;
    pi: 3.14159;
    ray: "a drop of golden sun";
    xmas: true;
    "xmas-fifth-day": {
      "calling-birds": "four";
      "french-hens": 3;
      "golden-rings": 5;
      partridges: { count: 1; location: "a pear tree" };
      "turtle-doves": "two";
    };
  };
  export type Document1 = {
    "calling-birds": ["huey", "dewey", "louie", "fred"];
    doe: "a deer, a female deer";
  };
  export type Document2 = { hello: "world" };
  export type All = [Document0, Document1, Document2];
}
```

#### Usage

```
Usage: ntro yaml [OPTIONS] <SOURCE_FILE>

Arguments:
  <SOURCE_FILE>  Path to a yaml file

Options:
  -o <OUTPUT_DIR>      Set the output directory, to where to save the *.d.ts file
  -q, --quiet          Disable logs
  -h, --help           Print help
```

### .env -> .ts

```env
NAME="value"
NEXT_PUBLIC_KEY="yoa",

# notice the type hint below
# @type 'a' | 'b'
NAME2=value
KEY = "value" # asdfa

keys= 'city'

# another type hint
# @type string
keys2 ='city'
```

`*.d.ts`

```ts
declare namespace NodeJS {
  interface ProcessEnv {
    KEY?: string;

    NAME?: string;

    NAME2?: string;

    NEXT_PUBLIC_KEY: string;

    keys?: string;

    keys2?: string;
  }
}
```

You can also generate a typescript module that parses environment
variables with extra type safety using zod. The advantage is that
type hint comment are collected and used to define the zod schema.

```ts
import z, { ZodTypeAny } from "zod";

const clientEnvSchemas = {
  NEXT_PUBLIC_KEY: z.string(),
};

const serverEnvSchemas = {
  ...clientEnvSchemas,
  NAME: z.string(),
  NAME2: z.enum(["a", "b"]),
  keys: z.string(),
  keys2: z.string(),
};

//... more implementation details are generated
```

#### Usage

```
Usage: ntro dotenv [OPTIONS] [SOURCE_FILES]...

Arguments:
  [SOURCE_FILES]...  Path(s) to some .env files

Options:
  -o <OUTPUT_DIR>                 Set the output directory, to where to save the env.d.ts file
  -q, --quiet                     Disable logs
  -z, --zod                       Generate a typescript module implementing a zod schema for env variables
  -w, --watch                     Wath for changes in the source files and rerun
  -p, --set-ts-config-path-alias  Update the project's tsconfig.json to include a path alias to the env.parsed.ts module that holds the zod schemas
  -h, --help                      Print help
```
