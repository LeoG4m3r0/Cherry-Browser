using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Windows.Forms;

internal sealed class CherrySetup : Form
{
    private readonly CheckBox desktopShortcut;
    private readonly Label status;
    private readonly ProgressBar progress;
    private readonly Button action;
    private const string AppVersion = "__APP_VERSION__";
    private const string ReleaseChannel = "__RELEASE_CHANNEL__";
    private bool installed;
    private string targetPath;
    private readonly string installFolder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "Cherry Browser");

    private CherrySetup()
    {
        Text = "Instalar Cherry Browser";
        StartPosition = FormStartPosition.CenterScreen;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = true;
        ClientSize = new Size(590, 390);
        BackColor = Color.FromArgb(27, 23, 26);
        ForeColor = Color.FromArgb(247, 242, 245);
        Font = new Font("Segoe UI", 10F);
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);

        var logo = new Panel { Location = new Point(28, 25), Size = new Size(78, 78), BackColor = Color.FromArgb(44, 30, 38) };
        logo.Paint += DrawCherry;
        Controls.Add(logo);

        var title = new Label { Text = "Cherry Browser", Location = new Point(124, 29), Size = new Size(420, 42), Font = new Font("Segoe UI Semibold", 23F, FontStyle.Bold), ForeColor = Color.White };
        Controls.Add(title);
        var subtitle = new Label { Text = "Navegação leve, com a sua cara.  ·  Versão " + AppVersion + "  ·  " + ReleaseChannel.ToUpperInvariant(), Location = new Point(126, 76), Size = new Size(420, 26), ForeColor = Color.FromArgb(193, 174, 184) };
        Controls.Add(subtitle);

        var line = new Panel { Location = new Point(28, 122), Size = new Size(534, 1), BackColor = Color.FromArgb(69, 55, 63) };
        Controls.Add(line);
        Controls.Add(new Label { Text = "Será instalado na sua pasta de usuário:", Location = new Point(30, 150), Size = new Size(520, 25) });
        Controls.Add(new Label { Text = installFolder, Location = new Point(30, 179), Size = new Size(525, 27), ForeColor = Color.FromArgb(255, 151, 184), AutoEllipsis = true });

        desktopShortcut = new CheckBox { Text = "Criar atalho na Área de Trabalho", Location = new Point(30, 222), Size = new Size(360, 28), Checked = true, ForeColor = ForeColor };
        Controls.Add(desktopShortcut);
        status = new Label { Text = "Pronto para instalar. Não é necessário acesso de administrador.", Location = new Point(30, 264), Size = new Size(530, 25), ForeColor = Color.FromArgb(193, 174, 184) };
        Controls.Add(status);
        progress = new ProgressBar { Location = new Point(30, 294), Size = new Size(530, 9), Minimum = 0, Maximum = 100, Value = 0, Style = ProgressBarStyle.Continuous };
        Controls.Add(progress);

        action = new Button { Text = "Instalar Cherry Browser", Location = new Point(322, 324), Size = new Size(238, 44), FlatStyle = FlatStyle.Flat, BackColor = Color.FromArgb(231, 87, 133), ForeColor = Color.FromArgb(35, 20, 27), Font = new Font("Segoe UI Semibold", 10F, FontStyle.Bold), Cursor = Cursors.Hand };
        action.FlatAppearance.BorderSize = 0;
        action.Click += InstallOrOpen;
        Controls.Add(action);
    }

    private static void DrawCherry(object sender, PaintEventArgs e)
    {
        var g = e.Graphics;
        g.SmoothingMode = SmoothingMode.AntiAlias;
        using (var stem = new Pen(Color.FromArgb(147, 103, 67), 5F))
        {
            stem.StartCap = LineCap.Round; stem.EndCap = LineCap.Round;
            g.DrawBezier(stem, 39, 41, 38, 28, 32, 18, 27, 13);
            g.DrawBezier(stem, 40, 40, 47, 26, 53, 17, 60, 14);
        }
        using (var leaf = new SolidBrush(Color.FromArgb(108, 199, 128)))
        using (var p = new GraphicsPath())
        {
            p.AddBezier(48, 22, 52, 8, 68, 8, 70, 15); p.AddBezier(70, 15, 65, 27, 55, 29, 48, 22); g.FillPath(leaf, p);
        }
        using (var fruit = new LinearGradientBrush(new Rectangle(12, 34, 55, 44), Color.FromArgb(255, 151, 181), Color.FromArgb(158, 17, 62), 45F))
        {
            g.FillEllipse(fruit, 11, 35, 35, 43); g.FillEllipse(fruit, 34, 35, 35, 43);
        }
        using (var shine = new SolidBrush(Color.FromArgb(110, Color.White)))
        { g.FillEllipse(shine, 18, 42, 6, 13); g.FillEllipse(shine, 41, 42, 6, 13); }
    }

    private void InstallOrOpen(object sender, EventArgs e)
    {
        if (installed)
        {
            try { Process.Start(new ProcessStartInfo(targetPath) { UseShellExecute = true }); Close(); }
            catch (Exception error) { MessageBox.Show(error.Message, "Cherry Browser", MessageBoxButtons.OK, MessageBoxIcon.Error); }
            return;
        }
        action.Enabled = false;
        try
        {
            status.Text = "Preparando a instalação…"; progress.Value = 15; Refresh(); Application.DoEvents();
            Directory.CreateDirectory(installFolder);
            targetPath = Path.Combine(installFolder, "Cherry-Browser.exe");
            status.Text = "Copiando o aplicativo…"; progress.Value = 48; Refresh(); Application.DoEvents();
            using (Stream input = Assembly.GetExecutingAssembly().GetManifestResourceStream("CherryBrowser.exe"))
            {
                if (input == null) throw new InvalidOperationException("O instalador não contém os arquivos do Cherry Browser.");
                using (var output = new FileStream(targetPath, FileMode.Create, FileAccess.Write, FileShare.None)) input.CopyTo(output);
            }
            status.Text = "Criando os atalhos…"; progress.Value = 78; Refresh(); Application.DoEvents();
            CreateShortcut(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.StartMenu), "Programs", "Cherry Browser.lnk"));
            if (desktopShortcut.Checked) CreateShortcut(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "Cherry Browser.lnk"));
            progress.Value = 100; status.Text = "Instalação concluída. Clique para abrir o navegador.";
            action.Text = "Concluir e abrir"; action.Enabled = true; installed = true;
        }
        catch (Exception error)
        {
            action.Enabled = true; status.Text = "Não foi possível concluir a instalação.";
            MessageBox.Show(error.Message, "Cherry Browser", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    private void CreateShortcut(string path)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path));
        Type shellType = Type.GetTypeFromProgID("WScript.Shell");
        if (shellType == null) throw new InvalidOperationException("O Windows não disponibilizou a criação de atalhos.");
        object shell = Activator.CreateInstance(shellType);
        object shortcut = shellType.InvokeMember("CreateShortcut", BindingFlags.InvokeMethod, null, shell, new object[] { path });
        Type linkType = shortcut.GetType();
        linkType.InvokeMember("TargetPath", BindingFlags.SetProperty, null, shortcut, new object[] { targetPath });
        linkType.InvokeMember("WorkingDirectory", BindingFlags.SetProperty, null, shortcut, new object[] { installFolder });
        linkType.InvokeMember("Description", BindingFlags.SetProperty, null, shortcut, new object[] { "Cherry Browser para Windows" });
        linkType.InvokeMember("Save", BindingFlags.InvokeMethod, null, shortcut, null);
        Marshal.ReleaseComObject(shortcut); Marshal.ReleaseComObject(shell);
    }

    [STAThread]
    private static void Main()
    {
        var arguments = Environment.GetCommandLineArgs();
        int parentPid;
        if (arguments.Length == 3 && arguments[1] == "--apply-update" && int.TryParse(arguments[2], out parentPid))
        {
            ApplyUpdate(parentPid);
            return;
        }
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new CherrySetup());
    }

    private static void ApplyUpdate(int parentPid)
    {
        try
        {
            try { using (var parent = Process.GetProcessById(parentPid)) parent.WaitForExit(); }
            catch (ArgumentException) { /* The browser may have exited before this helper started. */ }
            var installFolder = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "Cherry Browser");
            Directory.CreateDirectory(installFolder);
            var targetPath = Path.Combine(installFolder, "Cherry-Browser.exe");
            var stagingPath = targetPath + ".update";
            using (Stream input = Assembly.GetExecutingAssembly().GetManifestResourceStream("CherryBrowser.exe"))
            {
                if (input == null) throw new InvalidOperationException("O instalador não contém os arquivos do Cherry Browser.");
                using (var output = new FileStream(stagingPath, FileMode.Create, FileAccess.Write, FileShare.None)) input.CopyTo(output);
            }
            if (File.Exists(targetPath)) File.Replace(stagingPath, targetPath, null);
            else File.Move(stagingPath, targetPath);
            try
            {
                Type shellType = Type.GetTypeFromProgID("WScript.Shell");
                if (shellType != null)
                {
                    object shell = Activator.CreateInstance(shellType);
                    try
                    {
                        string startMenu = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.StartMenu), "Programs", "Cherry Browser.lnk");
                        object shortcut = shellType.InvokeMember("CreateShortcut", BindingFlags.InvokeMethod, null, shell, new object[] { startMenu });
                        Type linkType = shortcut.GetType();
                        linkType.InvokeMember("TargetPath", BindingFlags.SetProperty, null, shortcut, new object[] { targetPath });
                        linkType.InvokeMember("WorkingDirectory", BindingFlags.SetProperty, null, shortcut, new object[] { installFolder });
                        linkType.InvokeMember("Description", BindingFlags.SetProperty, null, shortcut, new object[] { "Cherry Browser para Windows" });
                        linkType.InvokeMember("Save", BindingFlags.InvokeMethod, null, shortcut, null);
                        Marshal.ReleaseComObject(shortcut);
                    }
                    finally { Marshal.ReleaseComObject(shell); }
                }
            }
            catch { /* A shortcut issue must not prevent reopening the updated browser. */ }
            Process.Start(new ProcessStartInfo(targetPath) { UseShellExecute = true, WorkingDirectory = installFolder });
        }
        catch (Exception error)
        {
            MessageBox.Show("A atualização automática não pôde ser concluída. O Cherry Browser não foi publicado nem alterado por esta tentativa; baixe o instalador e execute-o manualmente.\n\n" + error.Message,
                "Cherry Browser", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }
}


