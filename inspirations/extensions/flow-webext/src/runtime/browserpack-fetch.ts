import { FLOW_BROWSERPACK_ORIGIN } from "./catalog";
import { FlowBrowserStorage } from "./browser-storage";

let patched = false;

function normalize(url: string) {
  return url.replace(`${FLOW_BROWSERPACK_ORIGIN}/`, "");
}

export function installBrowserPackFetch(storage: FlowBrowserStorage) {
  if (patched) {
    return;
  }

  const nativeFetch = globalThis.fetch.bind(globalThis);

  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const requestUrl =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;

    if (requestUrl.startsWith(FLOW_BROWSERPACK_ORIGIN)) {
      const normalized = normalize(requestUrl);
      const [packKey, ...rest] = normalized.split("/");
      const filePath = rest.join("/");
      const record = await storage.readPackFile(packKey, filePath);

      if (!record) {
        return new Response(`Missing browser pack file: ${requestUrl}`, {
          status: 404,
        });
      }

      return new Response(record.bytes, {
        status: 200,
        headers: {
          "content-type": record.contentType,
          "x-flow-browserpack": packKey,
        },
      });
    }

    return nativeFetch(input as RequestInfo, init);
  };

  patched = true;
}
