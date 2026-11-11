import zombie.ZomboidFileSystem;
import zombie.scripting.ScriptManager;
import zombie.scripting.ScriptLoadMode;
import zombie.Lua.LuaManager.GlobalObject;

public class ScriptItemsProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-script-probe");
        ZomboidFileSystem.instance.init();
        zombie.core.random.RandStandard.INSTANCE.init();
        try { ScriptManager.instance.Load(); } catch (UnsatisfiedLinkError ex) { System.out.println("PROBE native-unavailable=" + ex.getClass().getSimpleName()); }
        var items = ScriptManager.instance.getAllItems();
        System.out.println("PROBE size=" + items.size());
        System.out.println("PROBE empty=" + items.isEmpty());
        System.out.println("PROBE first=" + items.get(0).getFullName());
        System.out.println("PROBE axe=" + ScriptManager.instance.getItem("Base.Axe").getFullName());
        System.out.println("PROBE same-manager=" + (GlobalObject.getScriptManager() == ScriptManager.instance));
        System.out.println("PROBE global-size=" + GlobalObject.getAllItems().size());
        System.out.println("PROBE contains-axe=" + items.contains(ScriptManager.instance.getItem("Base.Axe")));
        System.out.println("PROBE first-consistent=" + (items.get(0) == GlobalObject.getAllItems().get(0)));
        try { items.get(-1); } catch (IndexOutOfBoundsException ex) { System.out.println("PROBE negative-index=error"); }
        try { items.get(items.size()); } catch (IndexOutOfBoundsException ex) { System.out.println("PROBE end-index=error"); }
    }
}
