import java.lang.reflect.*;
import java.util.*;

public class TranslatorProbe {
    public static void main(String[] args) throws Exception {
        Class<?> t = Class.forName("zombie.core.Translator");
        @SuppressWarnings("unchecked")
        Map<String, String> ui = (Map<String, String>) t.getMethod("getUI").invoke(null);
        Method get = t.getMethod("getText", String.class, Object[].class);
        String[][] cases = {
            {"Hello %s", "Alex"}, {"%1 / %2", "one", "two"},
            {"%s %s", "one", "two"}, {"100%% %s", "done"},
            {"%2$s %1$s", "one", "two"}, {"%1 %2", "one"},
            {"%s %s", "one"}, {"plain", "extra"}, {"%s", "a%2"}, {"%1 %s", "one", "two"}, {"%1 %1", "A"}, {"%1%%", "50"}, {"%s %q", "one", "two"}, {"%5", "a", "b", "c", "d", "e"}, {"%s"}, {"literal 50%", "unused"}, {"%1 %s", "A"}, {"Wave %1 in : %2 (press Enter to skip)", "4", "9"}, {"average temperature %1. Minimum: %2. Maximum: %3. Humidity: %4%%...", "10", "5", "15", "50"}
        };
        for (String[] c : cases) {
            ui.put("UI_Probe", c[0]);
            Object[] values = Arrays.copyOfRange(c, 1, c.length, Object[].class);
            try {
                Object result = get.invoke(null, "UI_Probe", values);
                System.out.println(Arrays.toString(c) + " => " + result);
            } catch (InvocationTargetException ex) {
                System.out.println(Arrays.toString(c) + " => ERROR " + ex.getCause());
            }
        }
    }
}
