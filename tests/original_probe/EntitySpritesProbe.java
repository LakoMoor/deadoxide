import java.util.ArrayList;
import java.util.Collections;
import zombie.iso.SpriteDetails.IsoFlagType;
import zombie.iso.sprite.IsoSprite;
import zombie.iso.sprite.IsoSpriteManager;
import zombie.scripting.ScriptManager;
import zombie.scripting.entity.GameEntityScript;

// Inside the booted original: the entity scripts in ScriptManager order, and every sprite that carries the
// EntityScript flag with its EntityScriptName. Args: <scratch dir>
public class EntitySpritesProbe {
    public static void main(String[] a) throws Exception {
        ServerHarness.run(a[0], out -> {
            StringBuilder sb = new StringBuilder();
            for (GameEntityScript e : ScriptManager.instance.getAllGameEntities()) sb.append("E ").append(e.getName()).append('\n');
            ArrayList<String> names = new ArrayList<>(IsoSpriteManager.instance.namedMap.keySet());
            Collections.sort(names);
            for (String n : names) {
                IsoSprite s = IsoSpriteManager.instance.namedMap.get(n);
                if (s.getProperties().has(IsoFlagType.EntityScript) || s.getProperties().get("EntityScriptName") != null)
                    sb.append("S ").append(n).append(' ').append(s.getProperties().has(IsoFlagType.EntityScript) ? 1 : 0).append(' ').append(s.getProperties().get("EntityScriptName")).append('\n');
            }
            out.print(sb);
        });
    }
}
