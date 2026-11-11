import java.lang.reflect.InvocationTargetException;

public class FloatProbe {
    public static void main(String[] args) throws Exception {
        Class<?> platformType = Class.forName("se.krka.kahlua.j2se.J2SEPlatform");
        Object platform = platformType.getMethod("getInstance").invoke(null);
        Object env = platformType.getMethod("newTable").invoke(platform);
        Class<?> tableType = Class.forName("se.krka.kahlua.vm.KahluaTable");
        Class<?> compiler = Class.forName("se.krka.kahlua.luaj.compiler.LuaCompiler");
        var load = compiler.getMethod("loadstring", String.class, String.class, tableType);
        Class<?> threadType = Class.forName("se.krka.kahlua.vm.KahluaThread");
        Class<?> platformInterface = Class.forName("se.krka.kahlua.vm.Platform");
        Object thread = threadType.getConstructor(platformInterface, tableType).newInstance(platform, env);
        threadType.getField("debugOwnerThread").set(thread, Thread.currentThread());
        var pcall = threadType.getMethod("pcall", Object.class, Object[].class);
        for (String expr : args) {
            try {
                Object closure = load.invoke(null, "return " + expr, "probe", env);
                Object[] result = (Object[]) pcall.invoke(thread, closure, new Object[0]);
                System.out.println(expr + " => " + java.util.Arrays.toString(result));
            } catch (InvocationTargetException ex) {
                System.out.println(expr + " => ERROR " + ex.getCause());
            }
        }
    }
}
