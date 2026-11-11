import zombie.ZomboidFileSystem;
import zombie.core.Core;
import zombie.core.Languages;
import zombie.core.Translator;

public class RecipeNamesProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-translation-probe");
        ZomboidFileSystem.instance.init();
        Languages.instance.init();
        Core.getInstance().setOptionLanguageName(args[0]);
        Translator.loadFiles();
        for (int i = 1; i < args.length; i++) {
            String key = args[i];
            System.out.println("PROBE " + key + "=" + Translator.getRecipeName(key));
            System.out.println("TEXT " + key + "=" + Translator.getTextOrNull(key));
        }
    }
}
