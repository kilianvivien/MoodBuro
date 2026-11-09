import { useState, type FormEvent } from "react";
import {
  ArrowRight,
  Check,
  Globe2,
  LockKeyhole,
  LoaderCircle,
  Building2,
} from "lucide-react";
export function AddSite({
  pending,
  error,
  onSubmit,
}: {
  pending: boolean;
  error: string | null;
  onSubmit: (url: string) => void;
}) {
  const [url, setUrl] = useState("");
  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit(url.trim());
  }
  return (
    <div className="onboarding">
      <div className="step">
        <span className="step-dot" /> YOUR WORKSPACE, CONNECTED{" "}
        <span className="alpha">ALPHA</span>
      </div>
      <div className="hero-icon">
        <Building2 size={32} strokeWidth={1.5} />
        <span>
          <Check size={13} />
        </span>
      </div>
      <h1>
        A little closer to
        <br />
        your learning.
      </h1>
      <p className="intro">
        Bring your organisation’s Moodle into one calm space.
        <br />
        Your courses, your people, your familiar Moodle.
      </p>
      <form onSubmit={submit}>
        <label htmlFor="site-url">Your organisation’s Moodle URL</label>
        <div className="url-field">
          <Globe2 size={19} />
          <input
            id="site-url"
            type="text"
            inputMode="url"
            autoComplete="url"
            spellCheck={false}
            placeholder="https://moodle.your-organisation.edu"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            disabled={pending}
            required
            aria-describedby="url-help"
          />
        </div>
        <p id="url-help" className="hint">
          Use the base address you normally visit to access Moodle.
        </p>
        <button
          className="continue"
          disabled={pending || !url.trim()}
          type="submit"
        >
          {pending ? (
            <>
              <LoaderCircle className="spin" size={18} /> Waiting for sign-in…
            </>
          ) : (
            <>
              Connect to Moodle <ArrowRight size={18} />
            </>
          )}
        </button>
        {pending && (
          <p className="notice" role="status">
            Continue in your organisation’s sign-in window. Close that window to
            cancel.
          </p>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
      </form>
      <div className="privacy">
        <LockKeyhole size={16} />
        <p>
          You sign in directly with your organisation.
          <br />
          <span>MoodBuro never sees or stores your password.</span>
        </p>
      </div>
      <div className="setup-note">
        <span>01</span> Connect your organisation <i /> <span>02</span> Make
        yourself at home
      </div>
    </div>
  );
}
