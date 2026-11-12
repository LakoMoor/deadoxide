import java.lang.reflect.Field;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.iso.objects.*;

// Inside the booted original: CellLoader.LoadCellBinaryChunk for the chunks [x1..x2] x [y1..y2] (world chunk coords),
// with Rand replaced by java.util.Random(12345) so that random fields and the number of draws are comparable.
// The world's meta grid is swapped for one built by Create() alone (no basements; see MetaGridProbe).
// Args: <scratch dir> x1 y1 x2 y2
public class ChunkObjectsProbe {
    static String bits(float f) { return Integer.toHexString(Float.floatToIntBits(f)); }

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
            rf.set(RandStandard.INSTANCE, new java.util.Random(12345L));
            out.println("O lockedHouses=" + zombie.SandboxOptions.instance.lockedHouses.getValue());
            var gp = IsoChunk.class.getDeclaredMethod("getFromPool");
            gp.setAccessible(true);
            IsoCell cell = IsoWorld.instance.currentCell;
            StringBuilder sb = new StringBuilder();
            for (int wx = x1; wx <= x2; wx++) for (int wy = y1; wy <= y2; wy++) {
                IsoChunk ch = (IsoChunk) gp.invoke(null);
                ch.wx = wx; ch.wy = wy;
                boolean r = CellLoader.LoadCellBinaryChunk(cell, wx, wy, ch);
                sb.append("C ").append(wx).append(' ').append(wy).append(' ').append(r).append(' ').append(ch.getMinLevel()).append(' ').append(ch.getMaxLevel()).append('\n');
                for (int z = ch.getMinLevel(); z <= ch.getMaxLevel(); z++) for (int y = 0; y < 8; y++) for (int x = 0; x < 8; x++) {
                    IsoGridSquare sq = ch.getGridSquare(x, y, z);
                    if (sq == null) continue;
                    sb.append("S ").append(x).append(' ').append(y).append(' ').append(z).append(' ').append(sq.getRoomID());
                    for (int i = 0; i < sq.getObjects().size(); i++) {
                        IsoObject o = sq.getObjects().get(i);
                        sb.append(' ').append(o.getClass().getSimpleName()).append(':').append(o.sprite == null ? "null" : o.sprite.name);
                        if (o.attachedAnimSprite != null) for (var at : o.attachedAnimSprite) sb.append('+').append(at.parentSprite == null ? "null" : at.parentSprite.name);
                        if (sq.getSpecialObjects().contains(o)) sb.append('*');
                        if (o.getRenderYOffset() != 0.0F) sb.append('@').append(bits(o.getRenderYOffset()));
                        if (o instanceof IsoDoor d) sb.append("{o").append(d.IsOpen() ? 1 : 0).append('n').append(d.getNorth() ? 1 : 0).append('l').append(d.isLocked() ? 1 : 0).append('k').append(d.isLockedByKey() ? 1 : 0).append('}');
                        if (o instanceof IsoWindow w) sb.append("{n").append(w.getNorth() ? 1 : 0).append('l').append(w.isLocked() ? 1 : 0).append('p').append(w.isPermaLocked() ? 1 : 0).append('s').append(w.isSmashed() ? 1 : 0).append('}');
                        if (o instanceof IsoTree t) sb.append("{s").append(t.getSize()).append('}');
                        if (o.sprite != null && o.sprite.getProperties().has(zombie.iso.SpriteDetails.IsoFlagType.vegitation)) sb.append("{t").append(bits(o.tintr)).append(',').append(bits(o.tintg)).append(',').append(bits(o.tintb)).append('}');
                    }
                    sb.append('\n');
                }
                out.print(sb);
                sb.setLength(0);
            }
            out.println("X " + zombie.core.random.Rand.Next(1000000));
        });
    }
}
