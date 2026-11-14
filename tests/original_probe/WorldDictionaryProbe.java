import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

// The original zombie.world.DictionaryData.loadFromByteBuffer on a save's WorldDictionary.bin, then every table it
// filled, sorted by id (the maps are HashMaps). Run from the game root: WorldDictionaryProbe <WorldDictionary.bin>
public class WorldDictionaryProbe {
    static Object f(Object o, Class<?> c, String name) throws Exception {
        Field x = c.getDeclaredField(name);
        x.setAccessible(true);
        return x.get(o);
    }

    static String info(Object i) throws Exception {
        Class<?> c = Class.forName("zombie.world.DictionaryInfo");
        Object ov = f(i, c, "modOverrides");
        return f(i, c, "registryId") + "\t" + f(i, c, "fullType") + "\t" + f(i, c, "moduleName") + "\t" + f(i, c, "name") + "\t"
            + f(i, c, "modId") + "\t" + f(i, c, "isModded") + "\t" + f(i, c, "existsAsVanilla") + "\t" + f(i, c, "obsolete") + "\t"
            + f(i, c, "removed") + "\t" + (ov == null ? "null" : String.join(",", (List<String>) ov));
    }

    public static void main(String[] a) throws Exception {
        java.io.PrintStream out = System.out;
        System.setOut(System.err);
        zombie.ZomboidFileSystem.instance.setCacheDir("/tmp/pz-worlddictionary-probe");
        zombie.ZomboidFileSystem.instance.init();
        Class<?> dc = Class.forName("zombie.world.DictionaryData");
        Constructor<?> k = dc.getDeclaredConstructor();
        k.setAccessible(true);
        Object d = k.newInstance();
        Method m = dc.getDeclaredMethod("loadFromByteBuffer", ByteBuffer.class);
        m.setAccessible(true);
        m.invoke(d, ByteBuffer.wrap(Files.readAllBytes(Paths.get(a[0]))));
        StringBuilder sb = new StringBuilder();
        sb.append("N\t").append(f(d, dc, "nextInfoId")).append('\t').append(f(d, dc, "mextObjectNameId")).append('\t')
            .append(f(d, dc, "nextSpriteNameId")).append('\n');
        for (Object i : new TreeMap<>((Map<Short, ?>) f(d, dc, "itemIdToInfoMap")).values()) sb.append("I\t").append(info(i)).append('\n');
        sb.append("IT\t").append(((Map<?, ?>) f(d, dc, "itemTypeToInfoMap")).size()).append('\n');
        for (Object i : new TreeMap<>((Map<Short, ?>) f(d, dc, "entityIdToInfoMap")).values()) sb.append("E\t").append(info(i)).append('\n');
        sb.append("ET\t").append(((Map<?, ?>) f(d, dc, "entityTypeToInfoMap")).size()).append('\n');
        for (Map.Entry<Byte, String> e : new TreeMap<>((Map<Byte, String>) f(d, dc, "objectIdToNameMap")).entrySet())
            sb.append("O\t").append(e.getKey()).append('\t').append(e.getValue()).append('\n');
        sb.append("OT\t").append(((Map<?, ?>) f(d, dc, "objectNameToIdMap")).size()).append('\n');
        for (Map.Entry<Integer, String> e : new TreeMap<>((Map<Integer, String>) f(d, dc, "spriteIdToNameMap")).entrySet())
            sb.append("S\t").append(e.getKey()).append('\t').append(e.getValue()).append('\n');
        sb.append("ST\t").append(((Map<?, ?>) f(d, dc, "spriteNameToIdMap")).size()).append('\n');
        Class<?> sd = Class.forName("zombie.world.StringDictionary");
        Class<?> sr = Class.forName("zombie.world.StringDictionary$StringRegister");
        Class<?> si = Class.forName("zombie.world.DictionaryStringInfo");
        for (Object r : (ArrayList<?>) f(null, sd, "registers")) {
            sb.append("R\t").append(f(r, sr, "name")).append('\t').append(f(r, sr, "nextId")).append('\n');
            for (Object i : new TreeMap<>((Map<Short, ?>) f(r, sr, "idToStringMap")).values())
                sb.append("r\t").append(f(i, si, "registryId")).append('\t').append(f(i, si, "string")).append('\t').append(f(i, si, "isLoaded")).append('\n');
        }
        Class<?> cd = Class.forName("zombie.world.ScriptsDictionary");
        Class<?> cr = Class.forName("zombie.world.ScriptsDictionary$ScriptRegister");
        Class<?> ci = Class.forName("zombie.world.DictionaryScriptInfo");
        for (Object r : (ArrayList<?>) f(null, cd, "registers")) {
            sb.append("C\t").append(f(r, cr, "name")).append('\t').append(f(r, cr, "nextId")).append('\n');
            for (Object i : new TreeMap<>((Map<Short, ?>) f(r, cr, "idMap")).values())
                sb.append("c\t").append(f(i, ci, "registryId")).append('\t').append(f(i, ci, "version")).append('\t').append(f(i, ci, "name"))
                    .append('\t').append(f(i, ci, "isLoaded")).append('\n');
        }
        out.print(sb);
        out.flush();
    }
}
