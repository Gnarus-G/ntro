import z, { ZodTypeAny } from "zod";

const clientEnvSchemas = {
  dummy: z.string(),
};
const serverEnvSchemas = {
  dummy: z.string(),
};

const processEnv = {};

/* --- MAIN IMPLEMENTATION BELOW --- */

type ClientEnv = z.infer<z.ZodObject<typeof clientEnvSchemas>>;

export const clientEnv: ClientEnv = new Proxy({} as ClientEnv, {
  get(_, prop: string) {
    return lookupEnv(prop, clientEnvSchemas, () => {
      throw new Error(
        `${prop} is not defined for client side environment variables.`
      );
    });
  },
});

type Env = z.infer<z.ZodObject<typeof serverEnvSchemas>>;

export const env: Env = new Proxy({} as Env, {
  get(_, prop: string) {
    if (prop.startsWith("NEXT_PUBLIC_")) {
      return Reflect.get(clientEnv, prop);
    }
    return lookupEnv(prop, serverEnvSchemas, () => {
      throw new Error(
        `${prop} is not defined for server side environment variables.`
      );
    });
  },
});

const cache: Record<string, unknown> = {};

function lookupEnv<T extends Record<string, ZodTypeAny>>(
  prop: string,
  parsers: T,
  onNotFound: () => never
) {
  if (prop in cache) {
    return cache[prop];
  }

  try {
    if (prop in parsers) {
      const parsed = parsers[prop as keyof typeof parsers]?.parse(
        processEnv[prop as keyof typeof processEnv],
        { path: [prop] }
      );

      cache[prop] = parsed;

      return parsed;
    }
    onNotFound();
  } catch (e) {
    throw new BadEnvError(`failed to read ${prop} from proccess.env`, e);
  }
}

class BadEnvError extends Error {
  constructor(public message: string, public cause: unknown) {
    super(message);
    if (cause instanceof Error) {
      this.message = [message, cause].join("\n ↳ ");
    }
  }
}
