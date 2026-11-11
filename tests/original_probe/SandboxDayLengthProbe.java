import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import zombie.ZomboidFileSystem;
import zombie.Lua.LuaManager;
import zombie.SandboxOptions;
import zombie.core.random.RandStandard;

public class SandboxDayLengthProbe {
    public static void main(String[] args) throws Exception {
        ZomboidFileSystem.instance.setCacheDir("/tmp/pz-sandbox-probe");
        ZomboidFileSystem.instance.init(); RandStandard.INSTANCE.init(); LuaManager.init();
        byte[] bytes = Files.readAllBytes(Path.of(args[0]));
        var opts = SandboxOptions.instance;
        var buffer = ByteBuffer.wrap(bytes); opts.load(buffer);
        System.out.printf("FILE version=%d count=%d remaining=%d option=%d minutes=%d%n",
            ByteBuffer.wrap(bytes).getInt(4), ByteBuffer.wrap(bytes).getInt(12),
            buffer.remaining(), opts.dayLength.getValue(), opts.getDayLengthMinutes());
        byte[] key = "DayLength".getBytes(StandardCharsets.US_ASCII);
        int offset = -1;
        outer: for(int i=0;i<=bytes.length-key.length;i++) {
            for(int j=0;j<key.length;j++) if(bytes[i+j]!=key[j]) continue outer;
            if(bytes[i+key.length]==0 && bytes[i+key.length+1]==1) { offset=i+key.length+2; break; }
        }
        if(offset<0) throw new IllegalStateException("DayLength value not found");
        for(int i=1;i<=9;i++) {
            byte[] patch = bytes.clone(); patch[offset] = (byte)('0'+i);
            opts.load(ByteBuffer.wrap(patch));
            System.out.printf("LOAD %d=%d,%d%n",i,opts.dayLength.getValue(),opts.getDayLengthMinutes());
        }
        for(int i=1;i<=27;i++) {
            opts.dayLength.setValue(i);
            System.out.printf("OPTION %d=%d%n",i,opts.getDayLengthMinutes());
        }
    }
}
