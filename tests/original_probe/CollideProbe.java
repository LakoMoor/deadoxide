import java.lang.reflect.Field;
import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.HashMap;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.iso.SpriteDetails.IsoFlagType;

// Inside the booted original: loads the chunks [x1..x2] x [y1..y2] like ChunkObjectsProbe, then for every square runs
// RecalcProperties, doGridNav and RecalcAllWithNeighbours(true, getter) with a getter over exactly these chunks, and
// prints each square's property flags, object-type bits and collide/path/vision matrices.
// Args: <scratch dir> x1 y1 x2 y2
public class CollideProbe {
    public static void main(String[] a) throws Exception {
        int x1 = Integer.parseInt(a[1]), y1 = Integer.parseInt(a[2]), x2 = Integer.parseInt(a[3]), y2 = Integer.parseInt(a[4]);
        ServerHarness.run(a[0], out -> {
            IsoMetaGrid g = new IsoMetaGrid();
            g.Create();
            Field mg = IsoWorld.class.getDeclaredField("metaGrid");
            mg.setAccessible(true);
            mg.set(IsoWorld.instance, g);
            Field rf = zombie.core.random.RandAbstract.class.getDeclaredField("rand");
            rf.setAccessible(true);
            var gp = IsoChunk.class.getDeclaredMethod("getFromPool");
            gp.setAccessible(true);
            IsoCell cell = IsoWorld.instance.currentCell;
            HashMap<Long, IsoGridSquare> map = new HashMap<>();
            ArrayList<IsoGridSquare> all = new ArrayList<>();
            for (int wx = x1; wx <= x2; wx++) for (int wy = y1; wy <= y2; wy++) {
                rf.set(RandStandard.INSTANCE, new java.util.Random(12345L));
                IsoChunk ch = (IsoChunk) gp.invoke(null);
                ch.wx = wx; ch.wy = wy;
                CellLoader.LoadCellBinaryChunk(cell, wx, wy, ch);
                for (int z = ch.getMinLevel(); z <= ch.getMaxLevel(); z++) for (int y = 0; y < 8; y++) for (int x = 0; x < 8; x++) {
                    IsoGridSquare sq = ch.getGridSquare(x, y, z);
                    if (sq == null) continue;
                    map.put(((long) (sq.z + 64) << 48) | ((long) sq.x << 24) | sq.y, sq);
                    all.add(sq);
                }
            }
            Class<?> gs = Class.forName("zombie.iso.IsoGridSquare$GetSquare");
            Object getter = Proxy.newProxyInstance(gs.getClassLoader(), new Class<?>[]{gs}, (p, m, c) -> map.get(((long) ((int) c[2] + 64) << 48) | ((long) (int) c[0] << 24) | (int) c[1]));
            var nav = IsoGridSquare.class.getMethod("doGridNav", gs);
            var recalc = IsoGridSquare.class.getMethod("RecalcAllWithNeighbours", boolean.class, gs);
            for (IsoGridSquare sq : all) sq.RecalcProperties();
            for (IsoGridSquare sq : all) nav.invoke(sq, getter);
            for (IsoGridSquare sq : all) recalc.invoke(sq, true, getter);
            StringBuilder sb = new StringBuilder();
            for (IsoGridSquare sq : all) {
                sb.append("Q ").append(sq.x).append(' ').append(sq.y).append(' ').append(sq.z).append(' ').append(sq.haveRoof ? 1 : 0).append(' ');
                boolean first = true;
                for (IsoFlagType f : sq.getProperties().getFlagsList()) { if (!first) sb.append(','); first = false; sb.append(f.name()); }
                sb.append(' ');
                for (String k : sq.getProperties().getPropertyNames()) sb.append(k).append('=').append(sq.getProperties().get(k)).append(';');
                sb.append(' ').append(Long.toHexString(sq.hasTypes)).append(' ').append(Integer.toHexString(sq.collideMatrix)).append(' ').append(Integer.toHexString(sq.pathMatrix)).append(' ').append(Integer.toHexString(sq.visionMatrix)).append('\n');
            }
            out.print(sb);
        });
    }
}
