import type { ChangeEvent } from "react";

import { formatMessage } from "../i18n.ts";
import type { LoginInput } from "./form.ts";

export interface FieldsProps {
  readonly value: LoginInput;
  readonly privacyAdmin: string | null;
  readonly acknowledged: boolean;
  onChange(next: LoginInput): void;
  onAcknowledge(next: boolean): void;
}

function textField(
  name: keyof LoginInput,
  label: Parameters<typeof formatMessage>[0],
  props: FieldsProps,
  type: string,
) {
  const onChange = (event: ChangeEvent<HTMLInputElement>) =>
    props.onChange({ ...props.value, [name]: event.target.value });
  return (
    <label className="anybot-field">
      <span>{formatMessage(label)}</span>
      <input
        name={name}
        type={type}
        value={String(props.value[name])}
        onChange={onChange}
        autoCapitalize="none"
        autoCorrect="off"
      />
    </label>
  );
}

function CreateSection(props: FieldsProps) {
  return (
    <>
      {textField("invite", "login-invite-label", props, "text")}
      {props.privacyAdmin === null ? null : (
        <>
          <p data-role="privacy-note">
            {formatMessage("login-privacy-note", { admin: props.privacyAdmin })}
          </p>
          <label className="anybot-check">
            <input
              name="acknowledged"
              type="checkbox"
              checked={props.acknowledged}
              onChange={(event) => props.onAcknowledge(event.target.checked)}
            />
            <span>{formatMessage("login-privacy-ack")}</span>
          </label>
        </>
      )}
    </>
  );
}

export function LoginFields(props: FieldsProps) {
  return (
    <>
      {textField("url", "login-url-label", props, "url")}
      {textField("account", "login-account-label", props, "text")}
      {textField("password", "login-password-label", props, "password")}
      <label className="anybot-check">
        <input
          name="createAccount"
          type="checkbox"
          checked={props.value.createAccount}
          onChange={(event) =>
            props.onChange({ ...props.value, createAccount: event.target.checked })
          }
        />
        <span>{formatMessage("login-create-account")}</span>
      </label>
      {props.value.createAccount ? <CreateSection {...props} /> : null}
    </>
  );
}
