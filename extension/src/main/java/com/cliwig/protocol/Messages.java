package com.cliwig.protocol;

import com.google.gson.Gson;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.JsonSyntaxException;

/**
 * Thin JSON helpers for the CLIwig wire protocol.
 *
 * Request:  {"id":1,"c":"ping"} | {"id":2,"c":"set","k":"tempo","v":120}
 * Response: {"id":1,"ok":true,"result":{...}} | {"id":1,"ok":false,"error":{"code":"...","msg":"..."}}
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

    public static String ok(final JsonElement id, final JsonElement result) {
        final JsonObject resp = new JsonObject();
        if (id != null && !id.isJsonNull()) {
            resp.add("id", id);
        }
        resp.addProperty("ok", true);
        if (result != null && !result.isJsonNull()) {
            resp.add("result", result);
        }
        return GSON.toJson(resp);
    }

    public static String ok(final JsonElement id) {
        return ok(id, null);
    }

    public static String error(final JsonElement id, final String code, final String msg) {
        final JsonObject resp = new JsonObject();
        if (id != null && !id.isJsonNull()) {
            resp.add("id", id);
        }
        resp.addProperty("ok", false);
        final JsonObject err = new JsonObject();
        err.addProperty("code", code);
        err.addProperty("msg", msg != null ? msg : "");
        resp.add("error", err);
        return GSON.toJson(resp);
    }

    public static byte[] utf8(final String s) {
        return s.getBytes(java.nio.charset.StandardCharsets.UTF_8);
    }

    public static String utf8(final byte[] data) {
        return new String(data, java.nio.charset.StandardCharsets.UTF_8);
    }
}
