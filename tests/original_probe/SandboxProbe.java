import java.lang.reflect.Field;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import zombie.SandboxOptions;
import zombie.config.ConfigOption;

// Every sandbox option of the original in registration order: name, type, value as written to a save. First at the
// defaults, then after SandboxOptions.load of the given map_sand.bin, then the preset name.
// Run from the game root: SandboxProbe <map_sand.bin>
public class SandboxProbe {
    static void dump(java.io.PrintStream out, String tag, ArrayList<?> options) throws Exception {
        for (Object o : options) {
            ConfigOption c = ((SandboxOptions.SandboxOption) o).asConfigOption();
            out.println(tag + "\t" + c.getName() + "\t" + c.getType() + "\t" + c.getValueAsString());
        }
    }

    public static void main(String[] a) throws Exception {
        java.io.PrintStream out = System.out;
        System.setOut(System.err);
        zombie.ZomboidFileSystem.instance.setCacheDir("/tmp/pz-sandbox-probe");
        zombie.ZomboidFileSystem.instance.init();
        zombie.core.random.RandStandard.INSTANCE.init();
        zombie.Lua.LuaManager.init();
        SandboxOptions s = SandboxOptions.instance;
        Field f = SandboxOptions.class.getDeclaredField("options");
        f.setAccessible(true);
        ArrayList<?> options = (ArrayList<?>) f.get(s);
        dump(out, "D", options);
        s.load(ByteBuffer.wrap(Files.readAllBytes(Paths.get(a[0]))));
        dump(out, "L", options);
        out.println("P\t" + zombie.iso.IsoWorld.instance.getPreset());
        out.flush();
    }
}
