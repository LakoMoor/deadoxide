import zombie.ZomboidFileSystem;
import zombie.scripting.ScriptManager;

public class ItemRecipesProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-script-probe");
        ZomboidFileSystem.instance.init();
        zombie.core.random.RandStandard.INSTANCE.init();
        try { ScriptManager.instance.Load(); }
        catch (UnsatisfiedLinkError ex) { System.out.println("PROBE partial-load=" + ex.getClass().getSimpleName()); }
        for (String key : args) {
            var item = ScriptManager.instance.getItem(key);
            if (item == null) { System.out.println("PROBE " + key + "=missing"); continue; }
            var recipes = item.getLearnedRecipes();
            if (recipes == null) { System.out.println("PROBE " + key + "=null"); continue; }
            System.out.println("PROBE " + key + "=" + String.join("|", recipes));
        }
    }
}
