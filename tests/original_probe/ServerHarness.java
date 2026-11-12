import java.io.File;
import java.io.PrintStream;
import se.krka.kahlua.luaj.compiler.LuaCompiler;
import se.krka.kahlua.vm.JavaFunction;
import se.krka.kahlua.vm.LuaClosure;
import zombie.Lua.LuaEventManager;
import zombie.Lua.LuaManager;
import zombie.network.GameServer;

// Boots the ORIGINAL game as a headless dedicated server (the pinned jar plus natives for this OS on
// java.library.path) in a fresh scratch cache dir with a fixed world seed, then runs a probe on the server's main
// thread (the OnServerStarted event) and halts the JVM. Probes print to the PrintStream they are given (stdout);
// the game's own log goes to stderr.
//   java -Djava.library.path=<natives> -cp <probe classes>:projectzomboid.jar <Probe> <scratch dir> [args]
public final class ServerHarness {
    public interface Probe { void run(PrintStream out) throws Exception; }

    static void delete(File f) {
        File[] l = f.listFiles();
        if (l != null) for (File c : l) delete(c);
        f.delete();
    }

    public static void run(String scratch, Probe probe) throws Exception {
        PrintStream out = new PrintStream(new java.io.BufferedOutputStream(new java.io.FileOutputStream(java.io.FileDescriptor.out), 1 << 20), false, "UTF-8");
        System.setOut(System.err);
        File dir = new File(scratch);
        if (!dir.getName().startsWith("pz-oracle-")) throw new IllegalArgumentException("scratch dir must be named pz-oracle-*: " + scratch);
        delete(dir);
        Thread watcher = new Thread(() -> {
            try {
                // The world is ready once IsoWorld.init has made a cell; OnServerStarted fires on the main thread shortly after.
                while (zombie.iso.IsoWorld.instance == null || zombie.iso.IsoWorld.instance.currentCell == null || LuaManager.env == null) Thread.sleep(10);
                JavaFunction fn = (frame, n) -> {
                    int code = 0;
                    try {
                        probe.run(out);
                    } catch (Throwable t) {
                        t.printStackTrace();
                        code = 3;
                    }
                    out.flush();
                    Runtime.getRuntime().halt(code);
                    return 0;
                };
                LuaManager.env.rawset("__oracle_probe", fn);
                LuaClosure closure = LuaCompiler.loadstring("__oracle_probe()", "oracle_probe", LuaManager.env);
                LuaEventManager.AddEvent("OnServerStarted").callbacks.add(closure);
                Thread.sleep(120000);
                System.err.println("ORACLE: probe was never called");
                Runtime.getRuntime().halt(4);
            } catch (Throwable t) {
                t.printStackTrace();
                Runtime.getRuntime().halt(5);
            }
        });
        watcher.setDaemon(true);
        watcher.start();
        GameServer.main(new String[]{"-cachedir=" + dir.getAbsolutePath(), "-nosteam", "-adminpassword", "oracle", "-servername", "oracle", "-seed=DeadOxideOracle"});
    }
}
