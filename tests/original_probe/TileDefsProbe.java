import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Collections;
import zombie.ZomboidFileSystem;
import zombie.core.properties.PropertyContainer;
import zombie.iso.IsoWorld;
import zombie.iso.SpriteDetails.IsoFlagType;
import zombie.iso.sprite.IsoSprite;
import zombie.iso.sprite.IsoSpriteGrid;
import zombie.iso.sprite.IsoSpriteManager;

// Runs the original tile-definition loading sequence of IsoWorld.init (the seven vanilla files, no mods,
// no scripts, so EntityScriptName has no values) and dumps every sprite. Run from the game root:
//   java -cp zomboid-rs/target/original-probe:projectzomboid.jar TileDefsProbe > out.txt
public class TileDefsProbe {
    static Method m(String name, Class<?>... a) throws Exception {
        Method x = IsoWorld.class.getDeclaredMethod(name, a);
        x.setAccessible(true);
        return x;
    }

    static Object f(Object o, String name) throws Exception {
        Field x = o.getClass().getDeclaredField(name);
        x.setAccessible(true);
        return x.get(o);
    }

    public static void main(String[] args) throws Exception {
        java.io.PrintStream out = System.out;
        System.setOut(System.err);
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-tiledefs-probe");
        ZomboidFileSystem.instance.init();
        zombie.network.GameServer.server = true;
        IsoWorld w;
        try {
            w = new IsoWorld();
        } catch (Throwable t) {
            System.err.println("PROBE ctor-failed " + t);
            Field uf = sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
            uf.setAccessible(true);
            w = (IsoWorld) ((sun.misc.Unsafe) uf.get(null)).allocateInstance(IsoWorld.class);
        }
        IsoSpriteManager sm = IsoSpriteManager.instance;
        String[] files = {"newtiledefinitions.tiles", "tiledefinitions_erosion.tiles", "tiledefinitions_overlays.tiles",
            "tiledefinitions_b42chunkcaching.tiles", "tiledefinitions_noiseworks.patch.tiles", "jumbo_trees_big.tiles", "jumbo_trees.tiles"};
        int[] nums = {1, 2, 4, 5, -1, 6, 8};
        Method strings = m("LoadTileDefinitionsPropertyStrings", IsoSpriteManager.class, String.class, int.class);
        Method load = m("LoadTileDefinitions", IsoSpriteManager.class, String.class, int.class);
        for (int i = 0; i < files.length; i++) strings.invoke(w, sm, ZomboidFileSystem.instance.getMediaPath(files[i]), nums[i]);
        m("SetCustomPropertyValues").invoke(w);
        m("GenerateTilePropertyLookupTables").invoke(w);
        for (int i = 0; i < files.length; i++) load.invoke(w, sm, ZomboidFileSystem.instance.getMediaPath(files[i]), nums[i]);
        m("registerFakeJumboTree", IsoSpriteManager.class, int.class).invoke(w, sm, 7);

        ArrayList<String> names = new ArrayList<>(sm.namedMap.keySet());
        Collections.sort(names);
        StringBuilder sb = new StringBuilder();
        for (String n : names) {
            IsoSprite s = sm.namedMap.get(n);
            PropertyContainer p = s.getProperties();
            sb.append(n).append('\t').append(s.id).append('\t').append(s.getTileType()).append('\t');
            boolean first = true;
            for (IsoFlagType fl : p.getFlagsList()) { if (!first) sb.append(','); first = false; sb.append(fl.name()); }
            sb.append('\t');
            for (String k : p.getPropertyNames()) sb.append(k).append('=').append(p.get(k)).append(';');
            sb.append('\t');
            boolean[] b = {s.forceAmbient, s.solidfloor, s.canBeRemoved, s.attachedFloor, s.cutW, s.cutN, s.solid, s.solidTrans, s.invisible,
                s.alwaysDraw, s.forceRender, s.moveWithWind, s.isBush, s.treatAsWallOrder};
            for (boolean x : b) sb.append(x ? '1' : '0');
            sb.append('\t').append(s.firerequirement).append('\t').append(s.burntTile).append('\t').append(s.windType).append('\t').append(s.renderLayer);
            sb.append('\t').append(s.tilesetName).append('\t').append(s.tileSheetIndex);
            sb.append('\t').append(f(s, "parentObjectName")).append('\t').append(f(s, "fasciaEdge"));
            IsoSpriteGrid g = s.getSpriteGrid();
            if (g == null) sb.append("\t-");
            else sb.append('\t').append(g.getWidth()).append('x').append(g.getHeight()).append('x').append(g.getLevels()).append('@').append(g.getSpriteIndex(s))
                    .append(':').append(g.getAnchorSprite() == null ? "null" : g.getAnchorSprite().name);
            sb.append('\t').append(p.getSurface()).append(',').append(p.isSurfaceOffset() ? 1 : 0).append(p.isTable() ? 1 : 0).append(p.isTableTop() ? 1 : 0)
              .append(',').append(p.getStackReplaceTileOffset()).append(',').append(p.getItemHeight()).append(',').append(p.getSlopedSurfaceDirection())
              .append(',').append(p.getSlopedSurfaceHeightMin()).append(',').append(p.getSlopedSurfaceHeightMax());
            sb.append('\n');
        }
        out.print(sb);
        out.flush();
        System.err.println("PROBE sprites=" + names.size() + " intMap=" + sm.intMap.size());
    }
}
