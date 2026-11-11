import java.lang.reflect.*;
import java.util.*;

public class TranslatorValuesProbe {
    public static void main(String[] args) throws Exception {
        Class<?> t = Class.forName("zombie.core.Translator");
        @SuppressWarnings("unchecked")
        Map<String, String> ui = (Map<String, String>) t.getMethod("getUI").invoke(null);
        Method get = t.getMethod("getText", String.class, Object[].class);
        Object[][] cases = {
            {"%s", null}, {"%s", 1.0}, {"%s", 1.5}, {"%s", true},
            {"%1", null}, {"%d", 12.0}, {"%d", 12}, {"%f", 1.25},
            {"%b", false}, {"%S", "hello"}, {"[%8s]", "hi"}, {"[%.3s]", "abcdef"},
            {"%s %<s", "repeat"}, {"%2$s %s %s", "a", "b"},
            {"%1 %2", null, "x"}, {"%b", null}, {"%b", "false"}, {"%B", true}, {"%s", -0.0}, {"%s", 1e20}, {"%s", 0.0001}
        };
        for (int i = 0; i < cases.length; i++) {
            Object[] c = cases[i];
            ui.put("UI_Probe", (String)c[0]);
            Object[] values = Arrays.copyOfRange(c, 1, c.length);
            try {
                System.out.println(i + ":" + get.invoke(null, "UI_Probe", values));
            } catch (InvocationTargetException ex) {
                System.out.println(i + ":ERROR " + ex.getCause());
            }
        }
    }
}
