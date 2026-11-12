import zombie.ZomboidFileSystem;
import zombie.iso.IsoLot;
import zombie.iso.LotHeader;
import zombie.iso.MapFiles;

// Loads every chunk of the cells in [x1..x2] x [y1..y2] through the original IsoLot.load and prints one FNV-1a hash
// per cell over (minLevel, maxLevel, data.size, offsetInData for the cell's levels, data) of its 1024 chunks, plus the
// full content of chunk (16,16). Headers come from LotHeadersProbe's loader path.
// Run from the game root: LotPackProbe "Muldraugh, KY" x1 y1 x2 y2
public class LotPackProbe {
    static long h;
    static void mix(int v) { for (int i = 0; i < 4; i++) { h ^= (v >>> (8 * i)) & 255; h *= 0x100000001b3L; } }

    public static void main(String[] args) throws Exception {
        java.io.PrintStream out = System.out;
        System.setOut(System.err);
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-lotpack-probe");
        ZomboidFileSystem.instance.init();
        zombie.network.GameServer.server = true;
        zombie.core.random.RandStandard.INSTANCE.init();
        String dir = args[0];
        String path = ZomboidFileSystem.instance.getDirectoryString("media/maps/" + dir + "/");
        MapFiles mf = new MapFiles(dir, path, new java.io.File(path).getAbsolutePath(), 0);
        IsoLot.MapFiles.add(mf);
        mf.load();
        mf.postLoad();
        zombie.iso.IsoWorld.instance.setZombieVoronois(new java.util.ArrayList<>());
        zombie.iso.IsoMetaGrid grid = zombie.iso.IsoWorld.instance.getMetaGrid();
        grid.minX = mf.minX; grid.minY = mf.minY; grid.maxX = mf.maxX; grid.maxY = mf.maxY;
        java.lang.reflect.Field gf = zombie.iso.IsoMetaGrid.class.getDeclaredField("grid"); gf.setAccessible(true);
        gf.set(grid, new zombie.iso.IsoMetaCell[mf.maxX - mf.minX + 1][mf.maxY - mf.minY + 1]);
        Class<?> tc = Class.forName("zombie.iso.IsoMetaGrid$MetaGridLoaderThread");
        var ctor = tc.getDeclaredConstructor(zombie.iso.IsoMetaGrid.class, int.class); ctor.setAccessible(true);
        Object thread = ctor.newInstance(grid, mf.minY);
        var loadCell = tc.getDeclaredMethod("loadCell", MapFiles.class, int.class, int.class); loadCell.setAccessible(true);
        int x1 = Integer.parseInt(args[1]), y1 = Integer.parseInt(args[2]), x2 = Integer.parseInt(args[3]), y2 = Integer.parseInt(args[4]);
        StringBuilder sb = new StringBuilder();
        for (int cx = x1; cx <= x2; cx++) for (int cy = y1; cy <= y2; cy++) {
            if (!mf.hasCell(cx, cy)) continue;
            loadCell.invoke(thread, mf, cx, cy);
            LotHeader info = mf.getLotHeader(cx, cy);
            h = 0xcbf29ce484222325L;
            for (int lx = 0; lx < 32; lx++) for (int ly = 0; ly < 32; ly++) {
                IsoLot lot = IsoLot.get(mf, cx, cy, cx * 32 + lx, cy * 32 + ly, null);
                int n = (Math.min(info.maxLevel, 31) - Math.max(info.minLevel, -32) + 1) * 64;
                int base = (Math.max(info.minLevel, -32) - info.minLevel) * 64;
                mix(lot.minLevel); mix(lot.maxLevel); mix(lot.data.size());
                for (int i = 0; i < n; i++) mix(lot.offsetInData[base + i]);
                for (int i = 0; i < lot.data.size(); i++) mix(lot.data.get(i));
                if (lx == 16 && ly == 16) {
                    sb.append("D ").append(cx).append(' ').append(cy).append(' ').append(lot.minLevel).append(' ').append(lot.maxLevel);
                    for (int i = 0; i < n; i++) sb.append(' ').append(lot.offsetInData[base + i]);
                    sb.append(" |");
                    for (int i = 0; i < lot.data.size(); i++) sb.append(' ').append(lot.data.get(i));
                    sb.append('\n');
                }
                IsoLot.put(lot);
            }
            sb.append("L ").append(cx).append(' ').append(cy).append(' ').append(Long.toHexString(h)).append('\n');
            out.print(sb); sb.setLength(0);
        }
        out.flush();
    }
}
