import zombie.ZomboidFileSystem;
import zombie.core.Languages;
import zombie.core.Translator;

public class TranslationLoadProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-translation-probe");
        ZomboidFileSystem.instance.init();
        Languages.instance.init();
        var language = Languages.instance.getByName(args[0]);
        System.out.println("PROBE language=" + language);
        Translator.setLanguage(language);
        Translator.forLanguageStack(l -> System.out.println("PROBE stack=" + l.name()));
        zombie.core.Core.getInstance().setOptionLanguageName(args[0]);
        Translator.loadFiles();
        System.out.println("PROBE after=" + Translator.getLanguage());
        for (int i = 1; i < args.length; i++) {
            String key = args[i];
            System.out.println("PROBE " + key + "=" + Translator.getText(key));
            System.out.println("PROBE nullable=" + Translator.getTextOrNull(key));
        }
    }
}
