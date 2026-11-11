import zombie.ZomboidFileSystem;
import zombie.scripting.ScriptManager;
import zombie.scripting.objects.ItemType;

public class ItemPropertiesProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-script-probe");
        ZomboidFileSystem.instance.init();
        zombie.core.random.RandStandard.INSTANCE.init();
        try { ScriptManager.instance.Load(); }
        catch (UnsatisfiedLinkError ex) { System.out.println("PROBE partial-load=" + ex.getClass().getSimpleName()); }
        System.out.println("PROBE types=" + ItemType.LITERATURE + "," + ItemType.WEAPON + "," + ItemType.CLOTHING);
        for (String key : args) {
            var item = ScriptManager.instance.getItem(key);
            if (item == null) { System.out.println("PROBE " + key + "=null"); continue; }
            System.out.println("PROBE " + key + "=" + item.getFullName() + "," + item.getActualWeight()
                + "," + item.getObsolete() + "," + item.isHidden() + "," + item.getSkillTrained()
                + "," + item.getRecordedMediaCat() + "," + item.isItemType(ItemType.LITERATURE)
                + "," + item.isItemType(ItemType.WEAPON) + "," + item.isItemType(ItemType.CLOTHING));
        }
    }
}
