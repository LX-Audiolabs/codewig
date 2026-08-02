package com.codewig.protocol;

import com.google.gson.Gson;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.JsonSyntaxException;

/**
 * Thin JSON helpers for the Codewig wire protocol.
 *
 * Request:  {"c":"ping"} | {"c":"set","k":"tempo","v":120}
 * Response: {"ok":true,"result":{...}} | {"ok":false,"error":{"code":"...","msg":"..."}}
 */
public final class Messages {
    private static final Gson GSON = new Gson();

    private Messages() {
    }

    public static JsonObject parseRequest(final String json) throws JsonSyntaxException {
        final JsonElement el = JsonParser.parseString(json);
        if (!el.isJsonObject()) {
            throw new JsonSyntaxException("request must be a JSON object");
        }
        return el.getAsJsonObject();
    }

    public static String ok(final JsonElement result) {
        final JsonObject resp = new JsonObject();
        resp.addProperty("ok", true);
        if (result != null && !result.isJsonNull()) {
            resp.add("result", result);
        }
        return GSON.toJson(resp);
    }

    public static String ok() {
        return ok(null);
    }

    public static String error(final String code, final String msg) {
        final JsonObject resp = new JsonObject();
        resp.addProperty("ok", false);
        final JsonObject err = new JsonObject();
        err.addProperty("code", code);
        err.addProperty("msg", msg != null ? msg : "");
        resp.add("error", err);
        return GSON.toJson(resp);
    }

    public static String utf8(final byte[] data) {
        return new String(data, java.nio.charset.StandardCharsets.UTF_8);
    }
}
