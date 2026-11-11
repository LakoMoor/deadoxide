import zombie.ZomboidFileSystem;
import zombie.core.Languages;
import zombie.core.Translator;
import zombie.core.Core;

public class TranslationCategoriesProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-translation-probe");
        ZomboidFileSystem.instance.init();
        Languages.instance.init();
        Core.getInstance().setOptionLanguageName(args[0]);
        Translator.loadFiles();
        for (int i = 1; i < args.length; i++) {
            String key = args[i];
            System.out.println("PROBE text:" + key + "=" + Translator.getText(key));
            System.out.println("PROBE nullable:" + key + "=" + Translator.getTextOrNull(key));
            if (key.contains(".")) {
                System.out.println("PROBE item:" + key + "=" + zombie.Lua.LuaManager.GlobalObject.getItemNameFromFullType(key));
                System.out.println("PROBE evolved:" + key + "=" + zombie.Lua.LuaManager.GlobalObject.getItemEvolvedRecipeName(key));
            }
        }
    }
}
