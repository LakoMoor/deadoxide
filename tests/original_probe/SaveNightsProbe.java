import java.io.*;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import zombie.GameTime;
import zombie.core.random.RandStandard;
public class SaveNightsProbe {
    public static void main(String[] args) throws Exception {
        RandStandard.INSTANCE.init(); byte[] base=Files.readAllBytes(Path.of(args[0]));
        for(int value : new int[]{0,1,3,100}) {
            byte[] bytes=base.clone(); ByteBuffer.wrap(bytes).putInt(12,value);
            var t=new GameTime();
            try(var in=new DataInputStream(new BufferedInputStream(new ByteArrayInputStream(bytes)))) {
                t.load(in); System.out.printf("NIGHTS %d=%d,%d%n",value,t.getNightsSurvived(),in.available());
            }
        }
    }
}
