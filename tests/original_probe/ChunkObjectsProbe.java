import java.lang.reflect.Field;
import zombie.core.random.RandStandard;
import zombie.iso.*;
import zombie.iso.objects.*;

// Inside the booted original: CellLoader.LoadCellBinaryChunk for the chunks [x1..x2] x [y1..y2] (world chunk coords),
// with Rand replaced by java.util.Random(12345) before each chunk so that random fields and the number of draws are
// comparable. Chunks holding a TV or radio (item creation draws, not ported) print '?' for random fields and no X line.
// The world's meta grid is swapped for one built by Create() alone (no basements; see MetaGridProbe).
// Args: <scratch dir> x1 y1 x2 y2        -Doracle.trace=1 logs the call site of every draw to stderr.
public class ChunkObjectsProbe {
    static String bits(float f) { return Integer.toHexString(Float.floatToIntBits(f)); }

    static java.util.Random random() {
        if (System.getProperty("oracle.trace") == null) return new java.util.Random(12345L);
        return new java.util.Random(12345L) {
            @Override protected int next(int bits) {
                StringBuilder t = new StringBuilder("DRAW");
                int n = 0;
                for (StackTraceElement e : new Throwable().getStackTrace()) {
                    if (!e.getClassName().startsWith("zombie.") || e.getClassName().startsWith("zombie.core.random")) continue;
                    t.append(' ').append(e.getClassName().substring(e.getClassName().lastIndexOf('.') + 1)).append('.').append(e.getMethodName()).append(':').append(e.getLineNumber());
                    if (++n == 5) break;
                }
                System.err.println(t);
                return super.next(bits);
            }
        };
    }

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
            out.println("O lockedHouses=" + zombie.SandboxOptions.instance.lockedHouses.getValue());
            var gp = IsoChunk.class.getDeclaredMethod("getFromPool");
            gp.setAccessible(true);
            IsoCell cell = IsoWorld.instance.currentCell;
            StringBuilder sb = new StringBuilder();
            for (int wx = x1; wx <= x2; wx++) for (int wy = y1; wy <= y2; wy++) {
                rf.set(RandStandard.INSTANCE, random());
                IsoChunk ch = (IsoChunk) gp.invoke(null);
                ch.wx = wx; ch.wy = wy;
                boolean r = CellLoader.LoadCellBinaryChunk(cell, wx, wy, ch);
                boolean taint = false;
                for (int z = ch.getMinLevel(); z <= ch.getMaxLevel(); z++) for (int i = 0; i < 64; i++) {
                    IsoGridSquare sq = ch.getGridSquare(i % 8, i / 8, z);
                    if (sq != null) for (int k = 0; k < sq.getObjects().size(); k++) if (sq.getObjects().get(k) instanceof IsoWaveSignal) taint = true;
                }
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
                        if (o instanceof IsoDoor d) sb.append("{o").append(d.IsOpen() ? 1 : 0).append('n').append(d.getNorth() ? 1 : 0).append(taint ? "?" : "l" + (d.isLocked() ? 1 : 0) + "k" + (d.isLockedByKey() ? 1 : 0)).append('}');
                        if (o instanceof IsoWindow w) sb.append("{n").append(w.getNorth() ? 1 : 0).append(taint ? "?" : "l" + (w.isLocked() ? 1 : 0)).append('p').append(w.isPermaLocked() ? 1 : 0).append('s').append(w.isSmashed() ? 1 : 0).append('}');
                        if (o instanceof IsoTree t) sb.append("{s").append(t.getSize()).append('}');
                        if (o.sprite != null && o.sprite.getProperties().has(zombie.iso.SpriteDetails.IsoFlagType.vegitation)) sb.append("{t").append(taint ? "?" : bits(o.tintr) + "," + bits(o.tintg) + "," + bits(o.tintb)).append('}');
                    }
                    sb.append('\n');
                }
                if (!taint) sb.append("X ").append(zombie.core.random.Rand.Next(1000000)).append('\n');
                out.print(sb);
                sb.setLength(0);
            }
        });
    }
}
