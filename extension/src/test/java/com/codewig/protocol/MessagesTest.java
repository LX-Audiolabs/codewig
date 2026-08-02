package com.codewig.protocol;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonNull;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.JsonSyntaxException;
import org.junit.jupiter.api.Test;

class MessagesTest {

    @Test
    void okWithoutResultHasNoResultField() {
        final JsonObject resp = JsonParser.parseString(Messages.ok()).getAsJsonObject();
        assertTrue(resp.get("ok").getAsBoolean());
        assertFalse(resp.has("result"));
    }

    @Test
    void okWithResultEmbedsIt() {
        final JsonObject result = new JsonObject();
        result.addProperty("bpm", 120);
        final JsonObject resp = JsonParser.parseString(Messages.ok(result)).getAsJsonObject();
        assertTrue(resp.get("ok").getAsBoolean());
        assertEquals(120, resp.getAsJsonObject("result").get("bpm").getAsInt());
    }

    @Test
    void okWithJsonNullOmitsResult() {
        final JsonObject resp = JsonParser.parseString(Messages.ok(JsonNull.INSTANCE)).getAsJsonObject();
        assertTrue(resp.get("ok").getAsBoolean());
        assertFalse(resp.has("result"));
    }

    @Test
    void errorCarriesCodeAndMessage() {
        final JsonObject resp =
                JsonParser.parseString(Messages.error("BAD_REQUEST", "missing 'ref'")).getAsJsonObject();
        assertFalse(resp.get("ok").getAsBoolean());
        final JsonObject err = resp.getAsJsonObject("error");
        assertEquals("BAD_REQUEST", err.get("code").getAsString());
        assertEquals("missing 'ref'", err.get("msg").getAsString());
    }

    @Test
    void errorWithNullMessageBecomesEmptyString() {
        final JsonObject resp = JsonParser.parseString(Messages.error("INTERNAL", null)).getAsJsonObject();
        assertEquals("", resp.getAsJsonObject("error").get("msg").getAsString());
    }

    @Test
    void parseRequestAcceptsObject() {
        final JsonObject req = Messages.parseRequest("{\"c\":\"ping\"}");
        assertEquals("ping", req.get("c").getAsString());
    }

    @Test
    void parseRequestRejectsNonObject() {
        assertThrows(JsonSyntaxException.class, () -> Messages.parseRequest("[1,2]"));
        assertThrows(JsonSyntaxException.class, () -> Messages.parseRequest("\"ping\""));
        assertThrows(JsonSyntaxException.class, () -> Messages.parseRequest("42"));
    }

    @Test
    void parseRequestRejectsGarbage() {
        assertThrows(JsonSyntaxException.class, () -> Messages.parseRequest("{not json"));
    }

    @Test
    void utf8RoundtripsMultibyte() {
        final String s = "hällö € ♪ 日本語";
        assertEquals(s, Messages.utf8(s.getBytes(java.nio.charset.StandardCharsets.UTF_8)));
    }
}
