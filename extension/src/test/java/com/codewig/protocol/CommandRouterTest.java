package com.codewig.protocol;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codewig.bridge.ClipService;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.List;
import org.junit.jupiter.api.Test;

/**
 * Tests the Bitwig-free parts of {@link CommandRouter}: the private static
 * request-parsing helpers (via reflection) and the validation/error paths of
 * {@code handle()} that never touch a bridge service (router built with null
 * services — commands that would call one are not exercised here).
 */
class CommandRouterTest {

    // --- reflection plumbing -------------------------------------------------

    private static Object invoke(final String name, final Class<?>[] sig, final Object... args)
            throws Exception {
        final Method m = CommandRouter.class.getDeclaredMethod(name, sig);
        m.setAccessible(true);
        try {
            return m.invoke(null, args);
        } catch (final InvocationTargetException e) {
            if (e.getCause() instanceof Exception ex) {
                throw ex;
            }
            throw e;
        }
    }

    private static JsonObject json(final String s) {
        return JsonParser.parseString(s).getAsJsonObject();
    }

    private static boolean boolOr(final JsonObject req, final String key, final boolean def)
            throws Exception {
        return (boolean) invoke("boolOr", new Class<?>[] {JsonObject.class, String.class, boolean.class},
                req, key, def);
    }

    private static int intOr(final JsonObject req, final String key, final int def) throws Exception {
        return (int) invoke("intOr", new Class<?>[] {JsonObject.class, String.class, int.class},
                req, key, def);
    }

    private static String[] requireRefs(final JsonObject req) throws Exception {
        return (String[]) invoke("requireRefs", new Class<?>[] {JsonObject.class}, req);
    }

    @SuppressWarnings("unchecked")
    private static List<ClipService.NoteSpec> parseNotes(final JsonObject req) throws Exception {
        return (List<ClipService.NoteSpec>) invoke("parseNotes", new Class<?>[] {JsonObject.class}, req);
    }

    @SuppressWarnings("unchecked")
    private static List<ClipService.NoteSpec> parseNotesAllowEmpty(final JsonObject req) throws Exception {
        return (List<ClipService.NoteSpec>) invoke(
                "parseNotesAllowEmpty", new Class<?>[] {JsonObject.class}, req);
    }

    private static Integer optPositiveInt(final JsonObject req, final String key) throws Exception {
        return (Integer) invoke("optPositiveInt", new Class<?>[] {JsonObject.class, String.class},
                req, key);
    }

    // --- boolOr ---------------------------------------------------------------

    @Test
    void boolOrDefaultsWhenMissingOrNull() throws Exception {
        assertTrue(boolOr(json("{}"), "on", true));
        assertFalse(boolOr(json("{}"), "on", false));
        assertTrue(boolOr(json("{\"on\":null}"), "on", true));
    }

    @Test
    void boolOrReadsRealBooleans() throws Exception {
        assertTrue(boolOr(json("{\"on\":true}"), "on", false));
        assertFalse(boolOr(json("{\"on\":false}"), "on", true));
    }

    @Test
    void boolOrParsesStringForms() throws Exception {
        assertTrue(boolOr(json("{\"on\":\"on\"}"), "on", false));
        assertTrue(boolOr(json("{\"on\":\"TRUE\"}"), "on", false));
        assertTrue(boolOr(json("{\"on\":\"1\"}"), "on", false));
        assertFalse(boolOr(json("{\"on\":\"off\"}"), "on", true));
        assertFalse(boolOr(json("{\"on\":\"False\"}"), "on", true));
        assertFalse(boolOr(json("{\"on\":\"0\"}"), "on", true));
    }

    @Test
    void boolOrFallsBackToDefaultOnGarbage() throws Exception {
        assertTrue(boolOr(json("{\"on\":\"maybe\"}"), "on", true));
        assertFalse(boolOr(json("{\"on\":\"maybe\"}"), "on", false));
    }

    // --- intOr / optPositiveInt -----------------------------------------------

    @Test
    void intOrDefaultsWhenMissingOrNull() throws Exception {
        assertEquals(-1, intOr(json("{}"), "slot", -1));
        assertEquals(4, intOr(json("{\"beats\":null}"), "beats", 4));
        assertEquals(7, intOr(json("{\"slot\":7}"), "slot", -1));
    }

    @Test
    void optPositiveIntAcceptsPositiveRejectsZero() throws Exception {
        assertNull(optPositiveInt(json("{}"), "bars"));
        assertEquals(2, optPositiveInt(json("{\"bars\":2}"), "bars"));
        assertThrows(IllegalArgumentException.class, () -> optPositiveInt(json("{\"bars\":0}"), "bars"));
        assertThrows(IllegalArgumentException.class, () -> optPositiveInt(json("{\"bars\":-3}"), "bars"));
    }

    // --- requireRefs ------------------------------------------------------------

    @Test
    void requireRefsReadsArray() throws Exception {
        final String[] refs = requireRefs(json("{\"refs\":[\"kick\",\"bass\"]}"));
        assertEquals(2, refs.length);
        assertEquals("kick", refs[0]);
        assertEquals("bass", refs[1]);
    }

    @Test
    void requireRefsReadsCommaSeparatedString() throws Exception {
        final String[] refs = requireRefs(json("{\"refs\":\"kick, bass ,lead\"}"));
        assertEquals(3, refs.length);
        assertEquals("kick", refs[0]);
        assertEquals("bass", refs[1]);
        assertEquals("lead", refs[2]);
    }

    @Test
    void requireRefsRejectsMissingEmptyArrayAndEmptyString() {
        assertThrows(IllegalArgumentException.class, () -> requireRefs(json("{}")));
        assertThrows(IllegalArgumentException.class, () -> requireRefs(json("{\"refs\":[]}")));
        assertThrows(IllegalArgumentException.class, () -> requireRefs(json("{\"refs\":\"  \"}")));
    }

    // --- parseNotes ---------------------------------------------------------------

    @Test
    void parseNotesAppliesVelAndDurDefaults() throws Exception {
        final List<ClipService.NoteSpec> notes =
                parseNotes(json("{\"notes\":[{\"step\":0,\"key\":60}]}"));
        assertEquals(1, notes.size());
        final ClipService.NoteSpec n = notes.get(0);
        assertEquals(0, n.step());
        assertEquals(60, n.key());
        assertEquals(100, n.vel());
        assertEquals(1.0, n.dur());
        assertNull(n.pressure());
        assertNull(n.chance());
    }

    @Test
    void parseNotesReadsExplicitValues() throws Exception {
        final List<ClipService.NoteSpec> notes = parseNotes(json(
                "{\"notes\":[{\"step\":4,\"key\":36,\"vel\":90,\"dur\":2.5,\"chance\":0.5}]}"));
        final ClipService.NoteSpec n = notes.get(0);
        assertEquals(4, n.step());
        assertEquals(36, n.key());
        assertEquals(90, n.vel());
        assertEquals(2.5, n.dur());
        assertEquals(0.5, n.chance());
    }

    @Test
    void parseNotesRejectsBadShapes() {
        // missing field entirely
        assertThrows(IllegalArgumentException.class, () -> parseNotes(json("{}")));
        // not an array
        assertThrows(IllegalArgumentException.class, () -> parseNotes(json("{\"notes\":{}}")));
        // note not an object
        assertThrows(IllegalArgumentException.class, () -> parseNotes(json("{\"notes\":[1]}")));
        // missing step / key
        assertThrows(IllegalArgumentException.class,
                () -> parseNotes(json("{\"notes\":[{\"key\":60}]}")));
        assertThrows(IllegalArgumentException.class,
                () -> parseNotes(json("{\"notes\":[{\"step\":0}]}")));
        // empty array: parseNotes rejects, parseNotesAllowEmpty accepts
        assertThrows(IllegalArgumentException.class, () -> parseNotes(json("{\"notes\":[]}")));
    }

    @Test
    void parseNotesAllowEmptyAcceptsEmptyArray() throws Exception {
        assertTrue(parseNotesAllowEmpty(json("{\"notes\":[]}")).isEmpty());
    }

    // --- handle(): validation paths that never reach a service -------------------

    /** Router with null services — only commands failing validation are safe to call. */
    private static CommandRouter nullRouter() {
        return new CommandRouter(null, null, null, null, null, null, 9470);
    }

    private static JsonObject handle(final String requestJson) {
        return JsonParser.parseString(nullRouter().handle(json(requestJson))).getAsJsonObject();
    }

    private static void assertError(final JsonObject resp, final String code) {
        assertFalse(resp.get("ok").getAsBoolean());
        assertEquals(code, resp.getAsJsonObject("error").get("code").getAsString());
    }

    @Test
    void pingWorksWithoutServices() {
        final JsonObject resp = handle("{\"c\":\"ping\"}");
        assertTrue(resp.get("ok").getAsBoolean());
    }

    @Test
    void missingCommandFieldIsBadRequest() {
        assertError(handle("{}"), "BAD_REQUEST");
        assertError(handle("{\"c\":[1]}"), "BAD_REQUEST"); // non-primitive 'c'
        // numeric primitive passes the guard, then hits the default branch
        assertError(handle("{\"c\":42}"), "UNKNOWN_COMMAND");
    }

    @Test
    void unknownCommandIsReported() {
        assertError(handle("{\"c\":\"frobnicate\"}"), "UNKNOWN_COMMAND");
    }

    @Test
    void missingRequiredFieldsBecomeBadRequest() {
        assertError(handle("{\"c\":\"track.mute\"}"), "BAD_REQUEST"); // refs missing
        assertError(handle("{\"c\":\"track.rename\",\"ref\":\"a\"}"), "BAD_REQUEST"); // name missing
        assertError(handle("{\"c\":\"clip.launch\",\"track\":\"a\"}"), "BAD_REQUEST"); // slot missing
        assertError(handle("{\"c\":\"track.volume\",\"ref\":\"a\"}"), "BAD_REQUEST"); // v missing
        assertError(handle("{\"c\":\"clip.set-notes\",\"track\":\"a\",\"slot\":0}"), "BAD_REQUEST");
        assertError(handle("{\"c\":\"scene.launch\"}"), "BAD_REQUEST"); // ref missing
        assertError(handle("{\"c\":\"track.mute\",\"refs\":[\"a\"],\"bars\":0}"), "BAD_REQUEST");
    }

    @Test
    void setRequiresKnownKeyAndValue() {
        assertError(handle("{\"c\":\"set\"}"), "BAD_REQUEST");
        assertError(handle("{\"c\":\"set\",\"k\":\"tempo\"}"), "BAD_REQUEST");
        assertError(handle("{\"c\":\"set\",\"k\":\"nope\",\"v\":1}"), "UNKNOWN_KEY");
    }
}
